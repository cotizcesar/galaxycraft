//! Playable graphical client for the reconstructed StarCraft rules engine.
//!
//! ```text
//! cargo run --release --bin sc_view
//! cargo run --release --bin sc_view -- --chk map.chk --tiles assets/TileSet --races terran,protoss
//! ```
//!
//! Controls
//! --------
//! * left drag / shift-click: select units
//! * right click: move / attack / gather / enter transport
//! * A: attack-move (next right click), P: patrol, S: stop, H: hold
//! * B: build menu (worker selected), 1..8: build / train command
//! * Ctrl+1..9: set control group, 1..9 (no Ctrl): recall / command card
//! * arrows or WASD: scroll camera, mouse wheel: scroll, Home: base, Space: selection
//! * F: fog of war, L: leaderboard, F10: mission statistics, +/-: game speed
//! * Tab: idle worker, I: replay title intro, Esc: pause
//! * Alt+Enter: fullscreen on / off (starts fullscreen; `--windowed`)
//! * F10 / Alt-M menu, Alt-S save, Alt-L load, Alt-H / F11 help, Alt-O
//!   options, Ctrl/Alt-X exit, Ctrl/Alt-Q quit, Ctrl-M music, Ctrl-S sound,
//!   Ctrl/Alt-C centre, Shift-F2..F4 / F2..F4 map locations
//! * `--display wayland|x11` (or `GALAXYCRAFT_DISPLAY`): display backend
//!
//! Headless screens: `--shot FILE --shot-screen`
//! `intro|menu|episode|briefing|mission|ready|collections|collections-carbot|pause|leaderboard|stats|victory|defeat|score`.

// A player-facing window on Windows: no console (stderr goes nowhere; the
// missing-install case shows its own message window).
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Video option "Pillarbox": the game screens draw into a centred 4:3 box.
/// Stored outside `App::content_rect`, which is called from free helpers and
/// tests without an `App`; the frame update sets it from the option.
static GAME_PILLARBOX: AtomicBool = AtomicBool::new(false);

/// Unit ids covered by the shipped PreSale bonus skin (`anim/presale/`):
/// Scourge, Command Center, Infested Command Center, Hatchery, Lair, Hive
/// and Nexus (`hd2/anim/presale/skin.json`).
const PRESALE_SKIN_UNITS: [u16; 7] = [
    uid::SCOURGE,
    uid::COMMAND_CENTER,
    uid::INFESTED_COMMAND_CENTER,
    uid::HATCHERY,
    uid::LAIR,
    uid::HIVE,
    uid::NEXUS,
];

/// HD skin id for `type_id`: the Carbot unit collection (2) covers every
/// unit while it is equipped; otherwise "Show Bonus Skins" selects the
/// shipped PreSale set (1) for its seven units, and 0 is the base art.
fn unit_skin_id(type_id: u16, options: &GameOptions) -> u8 {
    if options.game_unit_skin == 2 {
        2
    } else if options.game_bonus_skins && PRESALE_SKIN_UNITS.contains(&type_id) {
        1
    } else {
        0
    }
}
use std::sync::mpsc::{Receiver, SyncSender};
use std::time::{Duration, Instant};

use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use sha2::{Digest, Sha256};

use galaxycraft::ai::SimpleAi;
use galaxycraft::aiscript::CampaignAi;
use galaxycraft::audio::{AudioPlayer, MusicBank, SoundBank};
use galaxycraft::campaign::BriefingPage;
use galaxycraft::data::GameData;
use galaxycraft::dds::DdsImage;
use galaxycraft::hotkey_data;
use galaxycraft::ids::unit as uid;
use galaxycraft::map::{PlayerSlot, Scenario, scenario_file_key};
use galaxycraft::menu::{
    GameDialog, GameDialogAction, GameOptions, HotkeyRace, LobbyRace, MenuScreen, OptionsTab,
};
use galaxycraft::net::{
    Announce, Lobby, Lockstep, Msg, TcpTransport, Transport, announcer, broadcast, client_turn,
    discover, discovery_socket,
};
use galaxycraft::render::{
    Framebuffer, Rgb, SpriteBank, TILE_PX, Tileset, asset_root, draw_unit_box, era_name,
};
use galaxycraft::replay::{RecAct, Recorder, Recording, SaveFile, state_hash};
use galaxycraft::retail::RetailInstall;
use galaxycraft::sfx::SfxTable;
use galaxycraft::sim::{Order, Race, SimUnit, World};
use galaxycraft::tech as tree;
use galaxycraft::ui::UiRect;
use galaxycraft::ui_art::{ConsoleArt, GlueArt};
use galaxycraft::video::{PosterFrame, RgbaFrame, VideoInfo, VideoStore};
use galaxycraft::voice::{VoiceBank, VoiceCue};

/// Optional gamepad backend (turns a controller into mouse/keyboard intent).
#[path = "sc_view/gamepad.rs"]
mod gamepad;
/// Multiplayer front end (Select Connection, Games, Create, lobby, score).
#[path = "sc_view/lift_fx.rs"]
mod lift_fx;
#[path = "sc_view/mp.rs"]
mod mp;
/// Controller radials and button prompts (Baldur's Gate 3 pad layout).
#[path = "sc_view/pad_ui.rs"]
mod pad_ui;

const HUD_H: i32 = 240;
/// Intro movie playback: 12 fps at 960 px wide keeps the decoder ahead of
/// the clock on a laptop CPU while still looking like film, not a slideshow.
/// The retail intro movies are authored at 15 fps (`smk/*.webm`); decoding
/// at a lower rate dropped one frame in five.
const INTRO_FPS: f32 = 15.0;
const INTRO_WIDTH: u32 = 960;
/// Smallest UI scale (content height / 480) that draws the world with the
/// retail HD art and screen-resolution fog.  It was 2.0, which sent 720p
/// (1.5) and 900p (1.875) to the SD sprites and 1x fog, so the game looked
/// different below 1080p.  Remastered draws the HD art at every supported
/// size; `scripts/resolution_diff.py` checks 720p..4K against 1440p.
const HD_WORLD_MIN_SCALE: f32 = 1.0;

/// Default window: the smallest supported 16:9 size (720p).  The front end
/// and the game are laid out in a 16:9 box; 4:3 defaults cut it off.
const VIEW_W: usize = 1280;
const VIEW_H: usize = 720;

/// Where the 16:9 drawing surface sits in a `w`x`h` window:
/// `(inner_w, inner_h, offset_x, offset_y)`.  Windows as wide as 16:9 or
/// wider (16:9, ultrawide) draw at the window size and pillarbox inside
/// `content_rect`.  Narrower windows (4:3, 16:10, 5:4) letterbox: the
/// surface is the 16:9 box that fits the width, centred vertically between
/// black bars, so nothing is laid out past the window edge (user report
/// 2026-11-05: at 1277x954 the Select Connection Ok / Cancel plates were cut
/// off on the right).
fn letterbox(w: usize, h: usize) -> (usize, usize, i32, i32) {
    let (w, h) = (w.max(1), h.max(1));
    if w * 9 >= h * 16 {
        return (w, h, 0, 0);
    }
    let ih = ((w as f32 * 9.0 / 16.0).round() as usize).clamp(1, h);
    (w, ih, 0, ((h - ih) / 2) as i32)
}

/// The drawn surface as the window shows it (letterbox bars included).
/// `None` when the surface already is the window.
fn presented(inner: &Framebuffer, w: usize, h: usize, offset: (i32, i32)) -> Option<Framebuffer> {
    if offset == (0, 0) && (inner.width as usize, inner.height as usize) == (w, h) {
        return None;
    }
    let mut out = Framebuffer::new(w, h);
    let mut pixels = Vec::new();
    present_into(inner, w, h, offset, &mut pixels);
    out.pixels = pixels;
    Some(out)
}

/// Window point -> drawing-surface point.
fn surface_point(offset: (i32, i32), x: i32, y: i32) -> (i32, i32) {
    (x - offset.0, y - offset.1)
}

/// Copy the drawn surface into a `w`x`h` window buffer at `offset`, with
/// black bars around it.
fn present_into(inner: &Framebuffer, w: usize, h: usize, offset: (i32, i32), out: &mut Vec<u32>) {
    out.clear();
    out.resize(w * h, 0);
    let (ox, oy) = (offset.0.max(0) as usize, offset.1.max(0) as usize);
    let iw = (inner.width as usize).min(w.saturating_sub(ox));
    for y in 0..(inner.height as usize).min(h.saturating_sub(oy)) {
        let src = y * inner.width as usize;
        let dst = (y + oy) * w + ox;
        out[dst..dst + iw].copy_from_slice(&inner.pixels[src..src + iw]);
    }
}

/// Retail campaign metadata materialised alongside the CHK file.
#[derive(Clone, Debug, Default)]
struct CampaignTag {
    episode: String,
    episode_name: String,
    mission: String,
    /// Cached copy of the retail `est*.txt` briefing script.
    script: Option<PathBuf>,
    /// Cached retail backdrop named by the script.
    background: Option<PathBuf>,
}

/// A row of the single-player `Play Custom` map browser.
#[derive(Clone, Debug)]
enum ClassicRow {
    Up,
    Folder(String),
    Map(usize),
}

#[derive(Clone, Debug)]
struct MapEntry {
    path: PathBuf,
    name: String,
    players: usize,
    campaign: Option<CampaignTag>,
}

impl MapEntry {
    fn new(path: PathBuf, name: String, players: usize) -> Self {
        Self {
            path,
            name,
            players,
            campaign: None,
        }
    }
}

/// Read the campaign manifest written by `RetailInstall::materialize`.
/// Campaign missions are placed before the melee maps, in retail episode
/// order; each one keeps the path of its briefing script and backdrop.
fn scan_campaign(cache: &Path) -> Vec<MapEntry> {
    let Ok(manifest) = std::fs::read_to_string(cache.join("campaign/manifest.tsv")) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for line in manifest.lines().skip(1) {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 9 {
            continue;
        }
        let chk = cache.join("campaign").join(fields[6]);
        let Ok(raw) = std::fs::read(&chk) else {
            continue;
        };
        let Ok(sc) = Scenario::parse(&raw) else {
            continue;
        };
        let players = sc
            .player_slots()
            .iter()
            .filter(|s| s.active)
            .count()
            .clamp(2, 8);
        let tag = CampaignTag {
            episode: fields[0].to_string(),
            episode_name: fields[1].to_string(),
            mission: fields[3].to_string(),
            script: (!fields[7].is_empty()).then(|| cache.join(fields[7])),
            background: (!fields[8].is_empty()).then(|| cache.join(fields[8])),
        };
        out.push(MapEntry {
            path: chk,
            name: format!("{}: {}", short_episode(&tag.episode_name), fields[4]),
            players,
            campaign: Some(tag),
        });
    }
    out
}

fn short_episode(name: &str) -> &'static str {
    match name {
        "Terran Campaign" => "T",
        "Zerg Campaign" => "Z",
        "Protoss Campaign" => "P",
        "Brood War: Terran" => "BW-T",
        "Brood War: Zerg" => "BW-Z",
        "Brood War: Protoss" => "BW-P",
        _ => "?",
    }
}

fn visit_maps(dir: &PathBuf, out: &mut Vec<MapEntry>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit_maps(&path, out);
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("chk") {
            continue;
        }
        let Ok(raw) = std::fs::read(&path) else {
            continue;
        };
        let Ok(sc) = Scenario::parse(&raw) else {
            continue;
        };
        let players = sc
            .player_slots()
            .iter()
            .filter(|s| s.active)
            .count()
            .clamp(2, 8);
        out.push(MapEntry::new(
            path.clone(),
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("map")
                .to_string(),
            players,
        ));
    }
}

fn scan_maps(dir: &PathBuf) -> Vec<MapEntry> {
    let mut out = Vec::new();
    visit_maps(dir, &mut out);
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Incremental replay player: rebuilds once, then applies due commands.
/// Save/replay directories: `$GALAXYCRAFT_SAVES`, else XDG/HOME cache.
/// `YYYY/MM/DD HH:MM` in UTC for a file timestamp.
///
/// The Replays browser shows when a file was written; the client has no
/// timezone database, so UTC is used and no local offset is implied.
fn format_timestamp(t: std::time::SystemTime) -> String {
    let secs = t
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Civil date from a day count (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    // Retail Replay Information shows `10/25/2026 10:07 PM` (130858).
    let (hour, minute) = (rem / 3600, (rem % 3600) / 60);
    let (h12, half) = match hour {
        0 => (12, "AM"),
        1..=11 => (hour, "AM"),
        12 => (12, "PM"),
        _ => (hour - 12, "PM"),
    };
    format!("{m:02}/{d:02}/{y:04} {h12}:{minute:02} {half}")
}

/// One row of the Load Replay information panel.
struct ReplayInfo {
    file: String,
    map: String,
    date: String,
    length: String,
    /// `(name, race)` per player.
    players: Vec<(String, Race)>,
}

impl ReplayInfo {
    /// The state of reference 130858 (`--shot-screen replays`): the retail
    /// `LastReplay.rep` of a Bottleneck game.
    fn reference() -> Self {
        Self {
            file: "LastReplay.rep".into(),
            map: "Bottleneck".into(),
            date: "10/25/2026 10:07 PM".into(),
            length: "00:11".into(),
            players: vec![
                ("Alpha Squadron".into(), Race::Terran),
                ("LinKa".into(), Race::Zerg),
            ],
        }
    }

    /// Metadata of a `.gcrep`: map, file date, length (mm:ss at 24 ticks
    /// per second) and the races; the local player carries `profile`.
    fn load(path: &Path, profile: &str) -> Option<Self> {
        let save = SaveFile::load(path)?;
        let date = std::fs::metadata(path)
            .and_then(|m| m.modified())
            .map(format_timestamp)
            .unwrap_or_default();
        let secs = save.tick / 24;
        let players = save
            .races
            .iter()
            .enumerate()
            .filter(|(_, r)| !matches!(r, Race::Neutral))
            .map(|(i, r)| {
                let name = if i == save.local_player {
                    profile.to_string()
                } else {
                    "Computer".to_string()
                };
                (name, *r)
            })
            .collect();
        Some(Self {
            file: path.file_name()?.to_str()?.to_string(),
            map: save.map_name,
            date,
            length: format!("{:02}:{:02}", secs / 60, secs % 60),
            players,
        })
    }
}

/// Folder rows above the replay files in the Load Replay tree (130858).
const REPLAY_FOLDERS: [&str; 3] = ["[Cloud]", "[Popular Maps]", "[AutoSave]"];

fn save_dirs() -> (PathBuf, PathBuf) {
    let base = std::env::var("GALAXYCRAFT_SAVES")
        .map(PathBuf::from)
        .unwrap_or_else(|_| galaxycraft::retail::user_cache_root());
    (base.join("saves"), base.join("replays"))
}

/// Decode a classic glue string (`rc3.json`): the value encodes the hotkey
/// before `\\u0004` and its position with `\\u0001`.
fn decode_glue_string(value: &str) -> String {
    let unescape = |value: &str| -> String {
        let mut out = String::new();
        let mut chars = value.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            match chars.peek().copied() {
                Some('n') => {
                    chars.next();
                    out.push('\n');
                }
                Some('t') => {
                    chars.next();
                    out.push('\t');
                }
                Some('\\') => {
                    chars.next();
                    out.push('\\');
                }
                Some('u') => {
                    chars.next();
                    let hex: String = chars.by_ref().take(4).collect();
                    if let Ok(code) = u32::from_str_radix(&hex, 16)
                        && let Some(ch) = char::from_u32(code)
                    {
                        out.push(ch);
                    }
                }
                _ => out.push(c),
            }
        }
        out
    };
    let decoded = unescape(value);
    if let Some(pos) = decoded.find('\u{4}') {
        let prefix = &decoded[..pos];
        let suffix = &decoded[pos + 1..];
        let mut text: String = prefix.chars().skip(1).collect();
        text.extend(suffix.chars().filter(|c| *c != '\u{1}'));
        text
    } else {
        decoded
            .chars()
            .filter(|c| !c.is_control() || *c == '\n')
            .collect()
    }
}

/// Decode a `locales/<locale>/assets/rez/<file>` string list into
/// `Key -> display text`, falling back to `enus` when the locale has no file.
fn load_json_strings(
    install: Option<&RetailInstall>,
    file: &str,
) -> std::collections::HashMap<String, String> {
    let Some(install) = install else {
        return Default::default();
    };
    let path = format!("locales/{}/assets/rez/{file}", install.locale());
    let bytes = install
        .read_by_path(&path)
        .or_else(|_| install.read_by_path(&format!("locales/enus/assets/rez/{file}")));
    match bytes {
        Ok(bytes) => parse_json_strings(&bytes),
        Err(_) => Default::default(),
    }
}

/// Parse one shipped `[{"Key","Value"}]` string list (`rc3.json`,
/// `gluall.json`) into decoded display text.
fn parse_json_strings(bytes: &[u8]) -> std::collections::HashMap<String, String> {
    let text = String::from_utf8_lossy(bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes));
    let Ok(entries) = serde_json::from_str::<Vec<serde_json::Value>>(&text) else {
        return Default::default();
    };
    entries
        .into_iter()
        .filter_map(|entry| {
            let key = entry.get("Key")?.as_str()?.to_string();
            let value = entry.get("Value")?.as_str()?;
            Some((key, decode_glue_string(value)))
        })
        .collect()
}

/// Classic `rc3.json` labels for the selected locale (`gluMain-*`, dialog
/// buttons, in-game menu entries).
fn load_ui_strings(install: Option<&RetailInstall>) -> std::collections::HashMap<String, String> {
    load_json_strings(install, "rc3.json")
}

/// Remastered CEF chrome strings (`gluall.json`): the Options panel, Create
/// Game, lobby and Hotkeys labels that the classic `rc3.json` lacks.
fn load_cef_strings(install: Option<&RetailInstall>) -> std::collections::HashMap<String, String> {
    load_json_strings(install, "gluall.json")
}

/// Last language chosen in Options; read on the next launch because retail
/// requires a restart to change the locale.
fn saved_locale_path() -> PathBuf {
    save_dirs()
        .0
        .parent()
        .map(|p| p.join("locale.txt"))
        .unwrap_or_else(|| galaxycraft::retail::user_cache_root().join("locale.txt"))
}

/// Pre-`GCPF` per-profile campaign progress (`GCPR`); read once to migrate.
fn progress_save_path() -> Option<PathBuf> {
    save_dirs().0.parent().map(|p| p.join("campaign.bin"))
}

/// Registry IDs, settings, statistics and campaign progress (`GCPF`), next
/// to `hotkeys.bin`.
fn profile_save_path() -> Option<PathBuf> {
    save_dirs().0.parent().map(|p| p.join("profiles.bin"))
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// `profiles.bin`; without one, the reference Registry (`REA`, `LinKa`, the
/// IDs of the retail captures) with any `campaign.bin` progress migrated.
/// `from_disk = false` (headless shots, scripts) always gives the reference
/// Registry, so captures never depend on the player's files.
fn load_profile_store(from_disk: bool) -> galaxycraft::profile::ProfileStore {
    if let Some(store) = profile_save_path()
        .filter(|_| from_disk)
        .and_then(|p| galaxycraft::profile::ProfileStore::load(&p))
    {
        return store;
    }
    let now = unix_now();
    let mut store = galaxycraft::profile::ProfileStore::default();
    for name in ["REA", "LinKa"] {
        let _ = store.create(name, now);
    }
    store.active = Some(0);
    if let Some(book) = progress_save_path()
        .filter(|_| from_disk)
        .and_then(|p| galaxycraft::campaign_progress::ProgressBook::load(&p))
    {
        store.import_progress_book(&book, now);
    }
    store
}

/// Rows the Load Saved Game list shows (218 UI px / 19 px pitch).
const SAVED_ROWS: usize = 11;
/// Visible Registry rows, shared by `draw_registry` and the click handler
/// (row geometry: `menu::LOGIN_DIALOG`).
const REGISTRY_ROWS: usize = 7;

/// First visible Registry row: the window that keeps `sel` on screen.
fn registry_top(sel: usize) -> usize {
    sel.saturating_sub(REGISTRY_ROWS - 1)
}

fn hotkey_save_path() -> Option<PathBuf> {
    save_dirs().0.parent().map(|p| p.join("hotkeys.bin"))
}

/// One placed piece of a unit's HD art (see `App::hd_unit_parts`).
struct HdPart {
    /// `images.dat` id (the `main.lit` index).
    image_id: u16,
    img: std::rc::Rc<galaxycraft::hd_sprites::HdImage>,
    frame: usize,
    x: f32,
    y: f32,
    below: bool,
}

/// A unit's main image, Init overlays and geyser smoke, placed on screen.
struct HdParts {
    main: HdPart,
    flip: bool,
    layers: Vec<HdPart>,
    smoke: Option<HdPart>,
}

struct ReplayPlayer {
    save: SaveFile,
    idx: usize,
    paused: bool,
}

impl ReplayPlayer {
    fn new(save: SaveFile) -> Self {
        Self {
            save,
            idx: 0,
            paused: false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NetRole {
    Host,
    Client,
}

/// A live LAN match inside the graphical client.
struct NetSession {
    role: NetRole,
    lock: Lockstep,
    /// Host: one transport per joined peer. Client: exactly one (the host).
    peers: Vec<TcpTransport>,
    desync: bool,
    last_hash: u64,
    /// The match setup, to rebuild the world after a reconnect.
    save: SaveFile,
    map_hash: [u8; 32],
    /// Host: the lobby listener stays open for rejoins.
    lobby: Option<Lobby>,
    /// Client: where the host listens.
    host_addr: String,
    /// Host: a dropped peer and when to stop waiting for it.
    waiting: Option<(u8, Instant)>,
    /// Client: reconnect schedule while the host is unreachable.
    reconnect: Option<galaxycraft::net::Backoff>,
}

/// How long a host keeps the match paused for a dropped peer.
const REJOIN_WAIT: Duration = Duration::from_secs(30);
/// In-match read timeout: longer than [`REJOIN_WAIT`], so peers that stay
/// connected do not give up on a host that is waiting for someone else.
const NET_READ_TIMEOUT: Duration = Duration::from_secs(40);

/// Remastered main-menu art layers, decoded from the player's own install.
///
/// These WebM "entry" videos are not text: they are the animated space
/// layers (fleet, battlecruiser, exit portal) that the retail engine
/// composites additively over the nebula.
/// One looping Remastered menu animation.
///
/// The shipped clips are ten seconds of VP9 with the transparency in
/// `BlockAdditional` chunks.  Decoding all 150 frames would cost hundreds of
/// megabytes, so a worker streams half-second chunks with alpha and the
/// player consumes them on its own wall clock, looping the clip.
struct MenuClip {
    path: PathBuf,
    info: VideoInfo,
    rx: Option<Receiver<Vec<Arc<galaxycraft::video::RgbaFrame>>>>,
    cancel: Arc<AtomicBool>,
    buffer: VecDeque<Arc<galaxycraft::video::RgbaFrame>>,
    current: Option<Arc<galaxycraft::video::RgbaFrame>>,
    decoded: u64,
    started: Instant,
    /// Playback rate (retail holds 15 fps source frames for 1/10.5 s).
    fps: f32,
    /// Rate the worker asks the decoder for; every authored frame is kept.
    source_fps: f32,
    chunk: usize,
    width: u32,
    /// Authored frames in one loop, from `start` to the end of the file.
    frames: usize,
    /// First second of the clip this player starts at (and loops back to).
    start: f32,
    /// `--menu-at` captures pin the first decoded frame instead of advancing
    /// with wall-clock time, so repeated renders are bit-identical.
    frozen: bool,
    /// When `frame` was last called: the clock pauses while the clip is not
    /// drawn (another screen is up), so it does not race ahead and then
    /// fast-forward through the queued frames on return.
    last_pull: Option<Instant>,
    /// Cached `RgbaFrame::has_alpha`; scanning 1.2M pixels per clip per frame
    /// was part of the menu's frame cost.
    has_alpha: bool,
    alpha_known: bool,
}

/// Retail advances the shipped 15 fps `glue/mainmenu` clips at a fixed
/// 10.5 fps: measured on the live client, the 151-frame Single/Multi clips
/// loop every 14.4 s and the 105-frame Exit clip every 10.0 s
/// (151 / 14.4 = 105 / 10.0 = 10.5).  Playing them at the WebM container
/// rate makes the loops 43 % too fast; resampling to 8 fps (the old value)
/// additionally dropped every other frame.
const MENU_CLIP_FPS: f32 = 10.5;
/// Native frame rate of the shipped WebM layers.  Decoding at this rate keeps
/// every authored frame; playback then holds each one for 1/10.5 s.
const MENU_CLIP_SOURCE_FPS: f32 = 15.0;
// Decode at the native width of the retail `glue/mainmenu` clips (1200 px)
// so the 1440p hover overlays keep their hairline grid instead of being
// upscaled from a 640 px intermediate.
const MENU_CLIP_WIDTH: u32 = 1200;
const MENU_CLIP_CHUNK: f32 = 0.5;
/// Retail loops each layer over its whole timeline (the last frame of
/// Single/Multi matches the first, and Exit's seam is a normal-motion step).
/// The old 2.5 s offset skipped the fleet entrance and made the seam visible.
const MENU_CLIP_START: f32 = 0.0;

/// Whether a main-menu clip is drawn this frame.  It is always pulled:
/// retail runs the mouse-over clip on the same clock as its idle clip
/// whether or not it is shown.  Screencast 2026-11-05 20:41:38: matched
/// against `singleon.webm` and `single.webm`, SingleOn shows the same frame
/// index as the fleet clip within 2 frames across six hovers and 9 s of
/// unhovered time.  A clock that paused while unhovered misses by up to 22
/// frames, and a restart would open on frame 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct MenuClipVisibility {
    drawn: bool,
}

fn menu_clip_visibility(idx: usize, is_hover: bool, hover: Option<usize>) -> MenuClipVisibility {
    MenuClipVisibility {
        drawn: !is_hover || hover == Some(idx),
    }
}

/// The shipped cursor GRP (80x84) is 4x the 640x480 UI art, i.e. authored
/// for a 1920-line canvas.  The live client draws it at `height / 1920`
/// (`ui_scale / 4`): measured 56-57 px tall at 1440p and 42-43 px at 1080p,
/// with the visible tip at the pointer.  Drawing the frames 1:1 (the old
/// behaviour) made the arrow 43 % too large at 1440p.
const CURSOR_ASSET_HEIGHT: f32 = 1920.0;
/// Retail cycles the five cursor frames at 6 fps (ten 60 Hz samples each).
const CURSOR_FPS: f32 = 6.0;

/// Colours of a main-menu label as `(rest of the word, hotkey glyph)`.
/// Hover only recolours the word: the glyph runs, faces and tracking stay
/// the idle ones for every entry, so no label moves when the pointer enters
/// it (user reports 2026-11-04 and 2026-11-05).
fn main_label_hover_colors(hovered: bool, normal: Rgb, bright: Rgb, hover: Rgb) -> (Rgb, Rgb) {
    if hovered {
        (hover, bright)
    } else {
        (normal, bright)
    }
}

/// Drop shadow of the glue labels and the build footer: `(offset px,
/// colour)`, one UI pixel down-right in opaque black.
fn label_drop_shadow(scale: f32) -> (i32, Rgb) {
    (scale.round().max(1.0) as i32, Rgb::new(0, 0, 0))
}

/// Authored-frame timeline for a menu layer: `(aligned_start, remaining
/// frames, playback seconds at 10.5 fps)`.  Kept pure so the retail rates
/// measured on the live client are unit-tested without a video decoder.
fn menu_clip_timeline(source_seconds: f32, start: f32) -> (f32, usize, f32) {
    let total_frames = (source_seconds.max(0.0) * MENU_CLIP_SOURCE_FPS)
        .round()
        .max(1.0) as usize;
    let max_start = total_frames.saturating_sub(1);
    let start_frame = ((start.max(0.0) * MENU_CLIP_SOURCE_FPS).round() as usize).min(max_start);
    let aligned_start = start_frame as f32 / MENU_CLIP_SOURCE_FPS;
    let frames = total_frames - start_frame;
    (aligned_start, frames, frames as f32 / MENU_CLIP_FPS)
}

impl MenuClip {
    /// Open the clip at an explicit second.  `--menu-at` uses this to take
    /// deterministic captures of the otherwise wall-clock animation.
    fn open_at(path: PathBuf, start: f32, frozen: bool) -> Option<Self> {
        let info = galaxycraft::video::probe_info(&path)?;
        let full = info.duration_ms? as f32 / 1000.0;
        if full <= 0.0 || info.pixel_width == 0 {
            return None;
        }
        // Timeline in authored frames, so the seam lands on the first frame
        // instead of drifting by the rounding of a seconds-based duration.
        // Retail holds each authored frame for 1/10.5 s, so a 151-frame
        // layer loops in 14.38 s (not the WebM's 10.07 s).
        let (start, frames, _duration) = menu_clip_timeline(full, start);
        let mut clip = Self {
            path,
            info,
            rx: None,
            cancel: Arc::new(AtomicBool::new(false)),
            buffer: VecDeque::new(),
            current: None,
            decoded: 0,
            started: Instant::now(),
            fps: MENU_CLIP_FPS,
            source_fps: MENU_CLIP_SOURCE_FPS,
            chunk: (MENU_CLIP_CHUNK * MENU_CLIP_SOURCE_FPS).round().max(1.0) as usize,
            width: MENU_CLIP_WIDTH,
            frames,
            start,
            frozen,
            last_pull: None,
            has_alpha: false,
            alpha_known: false,
        };
        clip.restart();
        Some(clip)
    }

    /// Start the streaming worker.  It keeps decoding the clip in a loop:
    /// the player wraps by simply consuming the next frame the worker
    /// produced, so the seam never waits for a fresh decoder/chunk (the old
    /// design stopped at the end and restarted the thread, which held the
    /// last frame on screen for a beat and looked like a cut).
    fn restart(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        self.buffer.clear();
        self.decoded = 0;
        self.started = Instant::now();
        let (tx, rx) = std::sync::mpsc::sync_channel(2);
        self.cancel = Arc::new(AtomicBool::new(false));
        let cancel = Arc::clone(&self.cancel);
        let path = self.path.clone();
        let info = self.info.clone();
        // Decode at the authored rate; playback (`self.fps`) is slower.
        let source_fps = self.source_fps;
        let width = self.width;
        let chunk = self.chunk;
        let clip_start = self.start;
        let total_frames = self.frames.max(1);
        std::thread::Builder::new()
            .name("menu-clip".into())
            .spawn(move || {
                let mut cursor = 0usize;
                loop {
                    if cancel.load(Ordering::Relaxed) {
                        return;
                    }
                    let count = chunk.min(total_frames - cursor).max(1);
                    let start = clip_start + cursor as f32 / source_fps;
                    let mut frames = galaxycraft::video::decode_rgba_frames_from_path(
                        &path, &info, start, count, source_fps, width,
                    );
                    if frames.is_empty() {
                        return;
                    }
                    // The decoder can hand back one extra frame at the tail;
                    // trim to the authored count so every loop has exactly
                    // the same frame sequence.
                    frames.truncate(count);
                    let got = frames.len();
                    // Hand ownership to the player as refcounted frames: the
                    // old Vec<RgbaFrame> move put a multi-megabyte copy on
                    // the render thread every time a chunk arrived.
                    let frames: Vec<Arc<galaxycraft::video::RgbaFrame>> =
                        frames.into_iter().map(Arc::new).collect();
                    if send_or_stop(&tx, frames, &cancel).is_err() {
                        return;
                    }
                    cursor += got;
                    if cursor >= total_frames {
                        cursor = 0;
                    }
                }
            })
            .ok();
        self.rx = Some(rx);
    }

    /// Frame to draw for this instant.
    fn frame(&mut self) -> Option<&galaxycraft::video::RgbaFrame> {
        if !self.frozen {
            let now = Instant::now();
            if let Some(gap) = self.last_pull.map(|t| now - t) {
                self.started += clip_pause_shift(gap);
            }
            self.last_pull = Some(now);
        }
        let mut want = if self.frozen {
            1
        } else {
            (self.started.elapsed().as_secs_f32() * self.fps).floor() as u64 + 1
        };
        // Pull at most one chunk per call; the sync channel bounds the
        // worker to two queued chunks (~6 MB at 640 px).  The first call
        // waits briefly for the decoder so headless captures show real art
        // instead of an empty layer.
        let first = self.current.is_none() && self.buffer.is_empty();
        if self.decoded < want
            && self.buffer.len() <= self.chunk * 2
            && let Some(rx) = &self.rx
        {
            // A frozen clip (`--menu-at` / `--menu-phase` captures) blocks
            // until the decoder delivers: the old 2 s poll gave up under
            // CPU load and drew an empty layer, so identical captures
            // differed.  Live playback never stalls the render thread.
            let got = if first && self.frozen {
                rx.recv_timeout(FROZEN_CLIP_WAIT).ok()
            } else {
                rx.try_recv().ok()
            };
            if let Some(frames) = got {
                // The playback clock starts with the first decoded
                // frame: counting the decoder's start-up time made the
                // first frames skip ahead.
                if first && !self.frozen && !frames.is_empty() {
                    self.started = Instant::now();
                    want = 1;
                }
                self.buffer.extend(frames);
            }
        }
        // Never fast-forward: when the clock is ahead of the decoder (a
        // stall), slide it back so playback resumes at the authored rate.
        if !self.frozen
            && self.buffer.is_empty()
            && let Some(behind) = clip_lag_shift(want, self.decoded, self.fps)
        {
            self.started += behind;
            want = self.decoded + 1;
        }
        let before = self.decoded;
        while self.decoded < want {
            match self.buffer.pop_front() {
                Some(f) => {
                    self.current = Some(f);
                    self.decoded += 1;
                }
                None => break,
            }
        }
        if clip_trace() && !self.frozen && self.decoded > before + 1 {
            // Diagnostic (`GC_CLIP_TRACE`): more than one authored frame in
            // a single draw, i.e. visible fast-forward.
            eprintln!(
                "clip {} skipped {} frames",
                self.path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?"),
                self.decoded - before - 1
            );
        }
        if !self.alpha_known
            && let Some(f) = self.current.as_ref()
        {
            self.has_alpha = f.has_alpha();
            self.alpha_known = true;
        }
        self.current.as_deref()
    }
}

fn send_or_stop(
    tx: &SyncSender<Vec<Arc<galaxycraft::video::RgbaFrame>>>,
    frames: Vec<Arc<galaxycraft::video::RgbaFrame>>,
    cancel: &AtomicBool,
) -> Result<(), ()> {
    // A bounded channel blocks when the player is behind; poll the cancel
    // flag while waiting so closing the client does not leak the worker.
    let mut frames = frames;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(());
        }
        match tx.try_send(frames) {
            Ok(()) => return Ok(()),
            Err(std::sync::mpsc::TrySendError::Full(back)) => {
                frames = back;
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => return Err(()),
        }
    }
}

/// A running classic-dialog swish (`menu::SWISH_STEPS_PER_SEC`).
#[derive(Clone)]
enum GlueSwish {
    /// The current screen's plates sliding in.
    In { started: Instant },
    /// The plates of `from` sliding out before the current screen shows.
    Out { from: MenuScreen, started: Instant },
}

/// Screens drawn by `draw_glue_list` (the plates that swish).
fn is_glue_list(screen: &MenuScreen) -> bool {
    matches!(
        screen,
        MenuScreen::Registry
            | MenuScreen::SavedGames
            | MenuScreen::Replays
            | MenuScreen::Connection
            | MenuScreen::Games
            | MenuScreen::MpCreate
            | MenuScreen::MpLobby
    )
}

/// Swish distances in UI px from the measured layout: the list plate's
/// right edge to the 16:9 box's left edge, the Ok plate's left edge to the
/// box's right edge, the Cancel plate's top to the box's bottom (1408, 932
/// and 372 px at 1440p, matching the recording's first steps).
const SWISH_LIST_UI: f32 = 1408.0 / 3.0;
const SWISH_OK_UI: f32 = 933.0 / 3.0;
const SWISH_CANCEL_UI: f32 = 372.0 / 3.0;
/// Campaign list (`glucmpgn`): the left console plate's right edge sits at
/// UI x 436 and the right plate starts at UI x 217, so they slide in from
/// 436 and 640 - 217 UI px away (recording 2026-11-07, 441.35-441.75 s).
const SWISH_EPISODE_LEFT_UI: f32 = 436.0;
const SWISH_EPISODE_RIGHT_UI: f32 = 640.0 - 217.0;

/// Screens whose plates slide in when they open: the classic glue lists
/// and the campaign list (user report 2026-11-07, recording 441.35 s).
fn slides_in(screen: &MenuScreen) -> bool {
    is_glue_list(screen) || matches!(screen, MenuScreen::Episode)
}

/// Steps elapsed since `started`.
fn swish_step(started: Instant) -> u32 {
    (started.elapsed().as_secs_f32() * galaxycraft::menu::SWISH_STEPS_PER_SEC) as u32
}

/// Draw order of the six main-menu clips as `(button, mouse-over)`.  The
/// hover clip goes under the idle clip for Single Player (capture 130704:
/// the fleet occludes the `SingleOn` grid) and over it for Multiplayer
/// (130712: the red `MultiOn` outline covers the battlecruiser's hull) and
/// Exit (130926: the `ExitOn` grid covers the vortex).  The clips all
/// carry alpha, so the order is what decides the overlap.
const MENU_CLIP_ORDER: [(usize, bool); 6] = [
    (0, true),
    (0, false),
    (1, false),
    (1, true),
    (7, false),
    (7, true),
];

/// `GC_CLIP_TRACE`: report menu clips whose decoder falls behind playback.
fn clip_trace() -> bool {
    static TRACE: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *TRACE.get_or_init(|| std::env::var_os("GC_CLIP_TRACE").is_some())
}

/// A pause between two `frame` calls longer than this is time the clip was
/// not on screen; its clock skips it.
const CLIP_PAUSE_GAP: Duration = Duration::from_millis(150);
/// Frames the clock may run ahead of the decoder before it is re-anchored.
const CLIP_MAX_LAG: u64 = 2;

/// Clock shift for a gap between two `frame` calls: a pause (the clip was
/// not drawn) is skipped entirely, ordinary frame gaps are kept.
fn clip_pause_shift(gap: Duration) -> Duration {
    if gap > CLIP_PAUSE_GAP {
        gap
    } else {
        Duration::ZERO
    }
}

/// Clock shift that brings the clock back to `decoded + 1` when it runs
/// more than [`CLIP_MAX_LAG`] frames ahead of an empty decoder buffer.
fn clip_lag_shift(want: u64, decoded: u64, fps: f32) -> Option<Duration> {
    (want > decoded + CLIP_MAX_LAG)
        .then(|| Duration::from_secs_f32((want - decoded - 1) as f32 / fps.max(1.0)))
}

/// Longest wait for the first frame of a frozen (capture) clip.
const FROZEN_CLIP_WAIT: Duration = Duration::from_secs(60);

/// The six Remastered main-menu button clips (idle + hover).
#[derive(Default)]
struct MenuLayers {
    single: Option<MenuClip>,
    single_on: Option<MenuClip>,
    multi: Option<MenuClip>,
    multi_on: Option<MenuClip>,
    exit: Option<MenuClip>,
    exit_on: Option<MenuClip>,
}

impl MenuLayers {
    fn clip(&mut self, idx: usize, hover: bool) -> Option<&mut MenuClip> {
        match (idx, hover) {
            (0, false) => self.single.as_mut(),
            (0, true) => self.single_on.as_mut(),
            (1, false) => self.multi.as_mut(),
            (1, true) => self.multi_on.as_mut(),
            (7, false) => self.exit.as_mut(),
            (7, true) => self.exit_on.as_mut(),
            _ => None,
        }
    }
}

/// The three episode-select slots of `glucmpgn`/`gluexpcmpgn`: a faded
/// portrait, the spinning disk hologram and the mouse-over variant, all
/// streamed from the shipped `glue/campaign` clips.
#[derive(Default)]
struct CampaignSlotClips {
    portrait: Option<MenuClip>,
    disk: Option<MenuClip>,
    on: Option<MenuClip>,
}

/// Layer set for one campaign set (original or Brood War).
#[derive(Default)]
struct CampaignLayers {
    expansion: bool,
    slots: [CampaignSlotClips; 3],
}

/// LAN lobby UI state (menu screen `Lan`).
struct LanUi {
    listener: Option<Lobby>,
    entries: Vec<(std::net::SocketAddr, Announce)>,
    sel: usize,
    status: String,
    /// Clients that joined the hosted lobby (host side).
    queue: Option<galaxycraft::net::JoinQueue>,
    announce_sock: Option<std::net::UdpSocket>,
    last_announce: Instant,
    /// Map the local player wants to play (index into `maps`).
    map_index: usize,
}

impl Default for LanUi {
    fn default() -> Self {
        Self {
            listener: None,
            entries: Vec::new(),
            sel: 0,
            status: "S: scan   H: host".to_string(),
            queue: None,
            announce_sock: None,
            last_announce: Instant::now() - Duration::from_secs(5),
            map_index: 0,
        }
    }
}

/// One menu input tick (keys pressed this frame, pointer, left button).
/// The Credits screen's CEF panel (see `galaxycraft::credits`): scroll
/// state, animation frames run since it appeared, held arrow keys, pending
/// wheel notches, the layout for the current box height and the rendered
/// lines (text coverage and text-shadow alpha, independent of the scroll).
#[derive(Default)]
struct CreditsView {
    scroll: galaxycraft::credits::CreditsScroll,
    frames: u64,
    since: Option<Instant>,
    keys: (bool, bool),
    wheel: i32,
    layout: Option<(i32, galaxycraft::credits::CreditsLayout)>,
    masks: std::collections::HashMap<usize, CreditMask>,
    /// CSS fallback faces after Eurostile Extended (`BlizzardGlobal`,
    /// `UDTypos510B_P_H`), read from the install when a line first needs a
    /// glyph Eurostile lacks.
    fallback: Option<Vec<galaxycraft::text::UiFont>>,
    /// The composed backdrop and panel for one framebuffer size.
    backdrop: Option<((usize, usize), Vec<u32>)>,
}

/// One rendered credit line: `x`/`top` place its top-left corner relative
/// to the list box's left edge and the line's baseline.
struct CreditMask {
    x: i32,
    top: i32,
    w: usize,
    h: usize,
    text: Vec<u8>,
    shadow: Vec<u8>,
}

/// One cargo cell of the selection panel: the `statdata` control that draws
/// it.  `cargo_slots` packs the loaded units into the 4 x 2 grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CargoSlot {
    control: i32,
}

/// Lay out a transport's cargo the way the selection panel does: small units
/// take 1 x 1 cells (`statdata` 25-32), two-space units a 1 x 2 column
/// (21-24), four-space units a 2 x 2 block (18-20).  The scan is
/// column-major, which keeps the one-space order the retail Bunker and
/// Dropship captures show.  A capacity-4 Bunker uses its centred 2 x 2 block
/// (27-30).
#[allow(clippy::needless_range_loop)]
fn cargo_slots(sizes: &[u8], capacity: u8) -> Vec<CargoSlot> {
    if capacity <= 4 {
        return sizes
            .iter()
            .take(4)
            .enumerate()
            .map(|(n, _)| CargoSlot {
                control: 27 + n as i32,
            })
            .collect();
    }
    let mut used = [[false; 4]; 2];
    let mut out = Vec::new();
    for &size in sizes {
        let (w, h) = match size {
            2 => (1, 2),
            4 | 8 => (2, 2),
            _ => (1, 1),
        };
        let mut placed = None;
        if (w, h) == (1, 1) {
            for col in 0..4 {
                for row in 0..2 {
                    if !used[row][col] {
                        placed = Some((col, row));
                        break;
                    }
                }
                if placed.is_some() {
                    break;
                }
            }
        } else if w == 1 {
            for col in 0..4 {
                if !used[0][col] && !used[1][col] {
                    placed = Some((col, 0));
                    break;
                }
            }
        } else {
            for col in 0..=2 {
                if !used[0][col] && !used[0][col + 1] && !used[1][col] && !used[1][col + 1] {
                    placed = Some((col, 0));
                    break;
                }
            }
        }
        let Some((col, row)) = placed else {
            continue;
        };
        for dy in 0..h {
            for dx in 0..w {
                used[row + dy][col + dx] = true;
            }
        }
        let control = if (w, h) == (1, 1) {
            25 + col as i32 * 2 + row as i32
        } else if h == 2 && w == 1 {
            21 + col as i32
        } else {
            match col {
                0 => 18,
                1 => 20,
                _ => 19,
            }
        };
        out.push(CargoSlot { control });
    }
    out
}

/// Facing units per game tick while "Smooth Unit Turning" rotates a sprite:
/// 24 leaves about 0.22 s for a 180-degree turn.
const SMOOTH_TURN_RATE: f32 = 24.0;

/// Move `current` towards `target` on the 256-step facing circle, taking the
/// shorter way around 0/256.  Returns a value in `0..256`.
fn smooth_step_facing(current: f32, target: u8, max_delta: f32) -> f32 {
    let current = current.rem_euclid(256.0);
    let target = target as f32;
    let mut diff = (target - current).rem_euclid(256.0);
    if diff > 128.0 {
        diff -= 256.0;
    }
    (current + diff.clamp(-max_delta, max_delta)).rem_euclid(256.0)
}

/// One integer hash for the deterministic environment particle field.
fn env_hash(seed: u32) -> u32 {
    let mut x = seed.wrapping_mul(0x9e37_79b9).wrapping_add(0x85eb_ca6b);
    x ^= x >> 15;
    x = x.wrapping_mul(0x2545_f491);
    x ^= x >> 13;
    x
}

/// Per-era look of the Environment Effects overlay.
#[derive(Clone, Copy)]
struct EnvSpec {
    colour: Rgb,
    alpha: u8,
    /// Vertical speed in screen px per game tick (negative rises).
    fall: f32,
    /// Horizontal wind in screen px per game tick.
    drift: f32,
    /// Sine sway amplitude in screen px.
    sway: f32,
    /// Base particle size in screen px at 1440p.
    size: f32,
}

fn env_spec(era: u16) -> EnvSpec {
    match era {
        // Badlands: warm dust motes.
        0 => EnvSpec {
            colour: Rgb::new(206, 184, 138),
            alpha: 70,
            fall: 0.12,
            drift: 0.35,
            sway: 5.0,
            size: 3.0,
        },
        // Space platform: sparse cool sparks.
        1 => EnvSpec {
            colour: Rgb::new(190, 208, 236),
            alpha: 55,
            fall: -0.08,
            drift: 0.12,
            sway: 3.0,
            size: 2.5,
        },
        // Installation: rising grey smoke.
        2 => EnvSpec {
            colour: Rgb::new(165, 163, 160),
            alpha: 45,
            fall: -0.18,
            drift: 0.20,
            sway: 9.0,
            size: 4.0,
        },
        // Ashworld: falling ash.
        3 => EnvSpec {
            colour: Rgb::new(158, 152, 146),
            alpha: 105,
            fall: 0.55,
            drift: 0.22,
            sway: 10.0,
            size: 3.0,
        },
        // Jungle: drifting spores.
        4 => EnvSpec {
            colour: Rgb::new(184, 232, 146),
            alpha: 60,
            fall: 0.12,
            drift: 0.16,
            sway: 13.0,
            size: 2.5,
        },
        // Desert: blowing sand.
        5 => EnvSpec {
            colour: Rgb::new(210, 190, 148),
            alpha: 85,
            fall: 0.06,
            drift: -1.5,
            sway: 2.0,
            size: 3.0,
        },
        // Ice: snowfall.
        6 => EnvSpec {
            colour: Rgb::new(236, 243, 255),
            alpha: 150,
            fall: 0.62,
            drift: 0.28,
            sway: 14.0,
            size: 3.0,
        },
        // Twilight: floating blue motes.
        _ => EnvSpec {
            colour: Rgb::new(176, 184, 240),
            alpha: 70,
            fall: 0.14,
            drift: 0.30,
            sway: 12.0,
            size: 2.5,
        },
    }
}

/// Blend one Environment Effects particle: the shipped texture supplies the
/// soft circular coverage, the era supplies colour.
fn draw_env_particle(
    fb: &mut Framebuffer,
    img: &galaxycraft::dds::DdsImage,
    x: i32,
    y: i32,
    size: i32,
    colour: Rgb,
    alpha: u8,
) {
    if size <= 0 || img.width == 0 || img.height == 0 {
        return;
    }
    let size = size.min(16);
    for dy in 0..size {
        for dx in 0..size {
            let sx = (((dx as f32 + 0.5) / size as f32) * img.width as f32) as usize;
            let sy = (((dy as f32 + 0.5) / size as f32) * img.height as f32) as usize;
            let tex = (img.pixel(sx.min(img.width - 1), sy.min(img.height - 1)) >> 24) & 0xff;
            let a = (tex * alpha as u32 / 255) as u8;
            if a == 0 {
                continue;
            }
            let px = x + dx;
            let py = y + dy;
            let dst = fb.get(px, py);
            fb.put(px, py, dst.blend(colour, a));
        }
    }
}

/// Words the retail "Mature Language Filter" masks.  There is no server-side
/// filter in this build, so this is a conservative local list used for
/// campaign text, subtitles and LAN chat; whole words only, case-insensitive.
const MATURE_WORDS: &[&str] = &[
    "ass", "asshole", "bastard", "bitch", "bollocks", "bullshit", "cock", "crap", "cunt", "damn",
    "dick", "douche", "fag", "faggot", "fuck", "goddamn", "hell", "nigger", "piss", "prick",
    "pussy", "shit", "slut", "twat", "whore",
];

fn censor_word(out: &mut String, word: &mut String) {
    if word.is_empty() {
        return;
    }
    let lower = word.to_ascii_lowercase();
    if MATURE_WORDS.contains(&lower.as_str()) {
        out.extend(word.chars().map(|_| '*'));
    } else {
        out.push_str(word);
    }
    word.clear();
}

/// Mask mature words in `text`, preserving punctuation, spacing and length.
fn mature_language_filter(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut word = String::new();
    for c in text.chars() {
        if c.is_ascii_alphabetic() {
            word.push(c);
        } else {
            censor_word(&mut out, &mut word);
            out.push(c);
        }
    }
    censor_word(&mut out, &mut word);
    out
}

/// Collections cells 0-2 are the Default/Silver/Gold console skins.
fn collection_console_skin(cell: usize) -> Option<u8> {
    match cell {
        0..=2 => Some(cell as u8),
        _ => None,
    }
}

/// Collections cells 5-10 are the announcer collections, Default first.
fn collection_announcer(cell: usize) -> Option<u8> {
    match cell {
        5..=10 => Some((cell - 5) as u8),
        _ => None,
    }
}

/// Hidden `ShowFPS` overlay text.
fn fps_label(fps: f32) -> String {
    format!("FPS {}", fps.round().max(0.0) as u32)
}

/// Sound tab "Building Sounds": gate voices emitted by structures.
fn building_sound_allowed(options: &GameOptions, is_building: bool) -> bool {
    options.building_sounds || !is_building
}

/// Hidden `ShowTurnRate` overlay text; `None` means no selected flingy.
fn turn_rate_label(rate: Option<u8>) -> String {
    match rate {
        Some(value) => format!("TURN {value}"),
        None => "TURN -".to_string(),
    }
}

/// APM from the command count and mission ticks.  The display stays at zero
/// during the opening minute, then uses the real elapsed minutes so an
/// opening flurry cannot read as 300 APM.
fn apm_value(actions: u32, ticks: u64) -> i64 {
    let seconds = galaxycraft::sim::real_seconds(ticks);
    if seconds < 60 {
        return 0;
    }
    (actions as f64 / (seconds as f64 / 60.0)).round() as i64
}

/// Window polling rate for the Video options.  minifb has no native VSync
/// switch, so "VSync" uses the common 60 Hz rate when the FPS cap is off;
/// both off means no sleep (0).
fn target_fps(options: &GameOptions) -> usize {
    if options.video_fps_cap {
        (100.0 + 200.0 * options.video_fps_limit.clamp(0.0, 1.0)).round() as usize
    } else if options.video_vsync {
        60
    } else {
        0
    }
}

/// One frame of in-game input in surface pixels (`App::handle_game_input`).
#[derive(Clone, Debug, Default)]
struct GameInput {
    pressed: Vec<Key>,
    ctrl: bool,
    alt: bool,
    shift: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    wheel: f32,
    /// Analog camera pan from a controller (game px this frame).
    pan: (i32, i32),
    mouse: Option<(i32, i32)>,
    lmb: bool,
    rmb: bool,
    /// Controller edges and stick aim for the same frame.
    pad: gamepad::PadFrame,
}

struct MenuInput {
    keys: Vec<Key>,
    mouse: Option<(i32, i32)>,
    mouse_down: bool,
    /// Wheel notches this tick (positive = towards the user, i.e. scroll
    /// the list down, as the live client does).
    wheel: i32,
    /// A Shift key is held (capitals in text fields).
    shift: bool,
}

/// One selectable sector of the controller command wheel.  It points at the
/// exact retail command-card slot the mouse and hotkeys already use.
#[derive(Clone, Debug)]
struct WheelEntry {
    /// Card set the slot lives in (`None` = the unit's own card, else a
    /// build card such as 239/242).
    set: Option<u16>,
    /// Zero-based card slot (0..9).
    slot: usize,
    /// Action the slot dispatches; kept for classification tests.
    #[allow(dead_code)]
    act: galaxycraft::buttons::Act,
    label: String,
    /// Tooltip text for the focused command (already stripped of the
    /// hotkey control byte).
    tooltip: String,
    icon: u16,
    enabled: bool,
    /// What A does on the slot (a card command, a shortcut, a group).
    action: pad_ui::PadAction,
}

/// One context page of the command wheel (Commands, Build, Train, ...).
#[derive(Clone, Debug)]
struct WheelPage {
    title: &'static str,
    entries: Vec<WheelEntry>,
}

/// Label of a command-card button for the wheel.  Retail stores the hotkey
/// as the first character of the tooltip, but the text table is not present
/// in every install, so use the data names of the unit/tech the action acts
/// on and short English labels for the orders.
fn wheel_label(
    world: &World,
    b: &galaxycraft::button_sets::Button,
    act: galaxycraft::buttons::Act,
) -> String {
    use galaxycraft::buttons::Act;
    let data = &world.data;
    let unit_label = |id: u16| {
        let name = data.unit_name(id);
        if name.is_empty() {
            "Action".to_string()
        } else {
            name.to_string()
        }
    };
    let tech_label = |id: u16| {
        let name = data.tech_name(id as u8);
        if name.is_empty() {
            "Upgrade".to_string()
        } else {
            name.to_string()
        }
    };
    match act {
        Act::Build
        | Act::DroneBuild
        | Act::Train
        | Act::TrainHangar
        | Act::Addon
        | Act::MorphUnit
        | Act::MorphBuilding => unit_label(b.act_var),
        Act::Research | Act::TargetTech | Act::StimPacks => tech_label(b.act_var),
        Act::Upgrade => {
            let name = data.upgrade_name(b.act_var as u8);
            if name.is_empty() {
                "Upgrade".to_string()
            } else {
                name.to_string()
            }
        }
        Act::Move => "Move".into(),
        Act::Stop | Act::StopOrMerge => "Stop".into(),
        Act::Attack => "Attack".into(),
        Act::Patrol => "Patrol".into(),
        Act::Hold => "Hold Position".into(),
        Act::Repair => "Repair".into(),
        Act::Gather => "Gather".into(),
        Act::ReturnCargo => "Return Cargo".into(),
        Act::Rally => "Set Rally Point".into(),
        Act::LiftOff => "Lift Off".into(),
        Act::Land => "Land".into(),
        Act::Load => "Load".into(),
        Act::Unload => "Unload".into(),
        Act::Siege => "Siege Mode".into(),
        Act::Unsiege => "Unsiege".into(),
        Act::Burrow => "Burrow".into(),
        Act::Unburrow => "Unburrow".into(),
        Act::Cloak => "Cloak".into(),
        Act::Decloak => "Decloak".into(),
        Act::MergeArchon => "Warp Archon".into(),
        Act::MergeDarkArchon => "Meld Dark Archon".into(),
        Act::Nuke => "Nuclear Strike".into(),
        Act::Heal => "Heal".into(),
        Act::Infest => "Infest".into(),
        Act::SpawnLarva => "Spawn Larva".into(),
        Act::RechargeShields => "Recharge Shields".into(),
        Act::PlaceBeacon => "Place Beacon".into(),
        Act::CancelLast
        | Act::CancelAddon
        | Act::CancelConstruction
        | Act::CancelMorph
        | Act::CancelTarget => "Cancel".into(),
        Act::Card | Act::Special | Act::Unclassified => "Action".into(),
    }
}

/// Build the single wheel page for the current selection.  A single unit
/// (or building) uses its current retail card; a group uses only the
/// commands every selected unit shares.  The result is capped at the nine
/// card slots, so the wheel never grows beyond the 3x3 layout.
fn wheel_page_for_selection(world: &World, units: &[SimUnit], card_set: Option<u16>) -> WheelPage {
    use galaxycraft::buttons::{Act, ButtonState, card_of_set};
    let title = if units.len() > 1 {
        "Group Commands"
    } else if units
        .first()
        .is_some_and(|u| world.data.unit(u.type_id).is_building())
    {
        "Building Commands"
    } else {
        "Unit Commands"
    };
    if units.is_empty() {
        return WheelPage {
            title,
            entries: Vec::new(),
        };
    }
    let single = units.len() == 1;
    let set_for = |_: &SimUnit| {
        if single {
            card_set.filter(|s| *s != 228)
        } else {
            None
        }
    };
    let cards: Vec<[Option<(galaxycraft::button_sets::Button, ButtonState)>; 9]> = units
        .iter()
        .map(|u| card_of_set(world, u, set_for(u)))
        .collect();
    let key = |b: &galaxycraft::button_sets::Button| (b.act, b.act_var, b.cond_var);
    let mut keys: Vec<(u32, u16, u16)> = Vec::new();
    for (b, _) in cards[0].iter().flatten() {
        let k = key(b);
        if !keys.contains(&k) {
            keys.push(k);
        }
    }
    keys.retain(|k| {
        cards
            .iter()
            .skip(1)
            .all(|card| card.iter().flatten().any(|(b, _)| key(b) == *k))
    });
    let mut entries = Vec::new();
    for k in keys.into_iter().take(9) {
        let Some((slot, b, _)) = cards[0].iter().enumerate().find_map(|(slot, e)| {
            e.as_ref()
                .map(|(b, st)| (slot, b, st))
                .filter(|(_, b, _)| key(b) == k)
        }) else {
            continue;
        };
        let enabled = cards.iter().all(|card| {
            card.iter()
                .flatten()
                .any(|(candidate, st)| key(candidate) == k && *st == ButtonState::Enabled)
        });
        let string_id = if !enabled && b.disabled_string != 0 {
            b.disabled_string
        } else {
            b.enabled_string
        };
        let raw = world
            .data
            .strings
            .get(string_id.saturating_sub(1))
            .unwrap_or("");
        entries.push(WheelEntry {
            set: set_for(&units[0]),
            slot,
            act: Act::of(b.act),
            label: wheel_label(world, b, Act::of(b.act)),
            tooltip: button_tooltip_text(raw),
            icon: b.icon,
            enabled,
            action: pad_ui::PadAction::Card,
        });
    }
    WheelPage { title, entries }
}

#[cfg(test)]
fn wheel_pages_for(world: &World, unit: &SimUnit) -> Vec<WheelPage> {
    use galaxycraft::button_sets::set_of;
    use galaxycraft::buttons::{Act, ButtonState, WheelCategory, card_of_set, state};
    let base = card_of_set(world, unit, None);
    let mut buckets: [Vec<WheelEntry>; 5] = Default::default();
    let index = |category: WheelCategory| {
        WheelCategory::ALL
            .iter()
            .position(|c| *c == category)
            .unwrap_or(0)
    };
    let entry = |set: Option<u16>, slot: usize, b: &galaxycraft::button_sets::Button| {
        let act = Act::of(b.act);
        let enabled = state(world, unit, b) == Some(ButtonState::Enabled);
        let string_id = if !enabled && b.disabled_string != 0 {
            b.disabled_string
        } else {
            b.enabled_string
        };
        let raw = world
            .data
            .strings
            .get(string_id.saturating_sub(1))
            .unwrap_or("");
        WheelEntry {
            set,
            slot,
            act,
            label: wheel_label(world, b, act),
            tooltip: button_tooltip_text(raw),
            icon: b.icon,
            enabled,
            action: pad_ui::PadAction::Card,
        }
    };
    // Build-card sets reachable from the unit's own card.
    let mut build_sets: Vec<u16> = Vec::new();
    for (b, _) in base.iter().flatten() {
        if Act::of(b.act) == Act::Card && b.act_var != 228 && !build_sets.contains(&b.act_var) {
            let set = card_of_set(world, unit, Some(b.act_var));
            if set
                .iter()
                .flatten()
                .any(|(b, _)| matches!(Act::of(b.act), Act::Build | Act::DroneBuild))
            {
                build_sets.push(b.act_var);
            }
        }
    }
    for (slot, entry_) in base.iter().enumerate() {
        let Some((b, _)) = entry_ else { continue };
        let act = Act::of(b.act);
        if act == Act::Card {
            continue;
        }
        let category = act.wheel_category();
        buckets[index(category)].push(entry(None, slot, b));
    }
    // The base card's build actions come first; then the real build cards
    // (all their entries are Build/DroneBuild; the Cancel card is skipped).
    for set_id in build_sets {
        for b in set_of(set_id as usize) {
            let act = Act::of(b.act);
            let slot = (b.pos as usize).wrapping_sub(1);
            if matches!(act, Act::Build | Act::DroneBuild)
                && slot < 9
                && state(world, unit, b).is_some()
            {
                buckets[index(WheelCategory::Build)].push(entry(Some(set_id), slot, b));
            }
        }
    }
    let mut pages = Vec::new();
    for (i, category) in WheelCategory::ALL.iter().enumerate() {
        if buckets[i].is_empty() {
            continue;
        }
        pages.push(WheelPage {
            title: category.title(),
            entries: std::mem::take(&mut buckets[i]),
        });
    }
    pages
}

/// Control under the pointer on a sub-screen (shared by every branch so
/// hover/click sounds and hit geometry stay consistent).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MenuHit {
    Ok,
    Cancel,
    Restore,
    Delete,
    Row(usize),
    Tab(usize),
    Cell(usize),
    OptSlider(usize),
    OptCheck(usize),
    /// The APM alert threshold field.
    OptInput,
    OptDrop(OptionsDrop),
    OptDropItem(usize),
    Race(usize),
    RacePick(usize, usize),
    Equip,
    Skip,
}

/// The two Options drop-downs whose open lists the reference captures show
/// (Video → Display Mode, Game → Language).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OptionsDrop {
    DisplayMode,
    Language,
}

/// Closed rect, open-list rect, row pitch and row count of an Options
/// drop-down, in 2560x1440 content pixels measured on references 17 and 19.
struct OptionsDropGeometry {
    closed: (f32, f32, f32, f32),
    list: (f32, f32, f32, f32),
    pitch: f32,
    count: usize,
}

fn options_drop_geometry(kind: OptionsDrop) -> OptionsDropGeometry {
    match kind {
        OptionsDrop::DisplayMode => OptionsDropGeometry {
            closed: (900.0, 373.5, 516.0, 45.0),
            list: (900.0, 418.5, 516.0, 45.0),
            pitch: 45.0,
            count: 3,
        },
        OptionsDrop::Language => OptionsDropGeometry {
            closed: (1456.0, 449.17, 516.0, 45.0),
            list: (1456.0, 494.17, 516.0, 43.0),
            pitch: 43.0,
            count: 13,
        },
    }
}

const DISPLAY_MODES: [&str; 3] = ["Windowed", "Windowed (Fullscreen)", "Fullscreen"];
const LANGUAGES: [&str; 13] = [
    "English",
    "Français",
    "Italiano",
    "Deutsch",
    "Español - España",
    "Español - Latino",
    "Português",
    "简体中文",
    "繁體中文",
    "한국어",
    "日本語",
    "Polski",
    "Русский",
];
/// CASC locale directory for each row of [`LANGUAGES`].
const LANGUAGE_CODES: [&str; 13] = [
    "enus", "frfr", "itit", "dede", "eses", "esmx", "ptbr", "zhcn", "zhtw", "kokr", "jajp", "plpl",
    "ruru",
];

fn language_index(code: &str) -> usize {
    LANGUAGE_CODES.iter().position(|c| *c == code).unwrap_or(0)
}

/// Retail footer/input tag: `enUS`, `esMX`, `zhCN`, ...
fn locale_tag(code: &str) -> String {
    let bytes = code.as_bytes();
    if bytes.len() >= 4 {
        format!(
            "{}{}",
            code[..2].to_ascii_lowercase(),
            code[2..4].to_ascii_uppercase()
        )
    } else {
        code.to_ascii_uppercase()
    }
}

/// Cache key of the Collections plate: window size, scroll offset, selected
/// cell and equipped unit skin (all change pixels baked into the layer).
type CollectionsLayerKey = (i32, i32, i32, usize, u8);

struct App {
    data: Arc<GameData>,
    scenario: Scenario,
    world: World,
    tileset: Tileset,
    sprites: SpriteBank,
    sounds: SoundBank,
    audio: AudioPlayer,
    /// Last value sent to the mixer for the background-audio checkbox.
    audio_background: bool,
    /// Materialised retail root (tileset, unit art, console, glue, music).
    assets_root: PathBuf,
    /// Retail console art for the human player's race.
    console: Option<ConsoleArt>,
    /// Retail main-menu and score surfaces.
    glue: Option<GlueArt>,
    /// Remastered front-end face (`webui/dist/lib/fonts/eurostileext-reg.otf`).
    ui_font: Option<galaxycraft::text::UiFont>,
    /// Glue faces (`font/eurostile-reg.ttf`, `font/eurostileext-reg.ttf`):
    /// the native glue renders from the TTFs, kern pairs included.
    glue_font: Option<galaxycraft::text::UiFont>,
    glue_font_ext: Option<galaxycraft::text::UiFont>,
    /// `font/bl.ttf` (BlizzardGlobal): `RC_SMALLTEXT` and plain buttons.
    glue_font_small: Option<galaxycraft::text::UiFont>,
    /// In-game HUD text face (`font/blizzard-regular.ttf`, see `GlueFace::Hud`).
    hud_font: Option<galaxycraft::text::UiFont>,
    /// Wide face for the lit main-menu labels (Eurostile Extended).
    ui_font_hover: Option<galaxycraft::text::UiFont>,
    /// Face used by the profile/build footer (HTML info text).
    ui_font_info: Option<galaxycraft::text::UiFont>,
    /// Optional system CJK/Cyrillic fallback for the language list (the
    /// shipped Eurostile face has no Chinese, Korean, Japanese or Russian
    /// glyphs).  Loaded on first use and absent on systems without one.
    language_font: Option<galaxycraft::text::UiFont>,
    language_font_loaded: bool,
    /// Retail Ogg music (decoded lazily) and the complete voice sets.
    music: MusicBank,
    voices: VoiceBank,
    /// Menu/ready-room music rotation.
    music_track: String,
    music_started: Instant,
    music_phase: u8,
    /// Race used for the currently loaded console.
    console_race: Race,
    /// Title/publisher intro before the main menu.
    intro: bool,
    intro_started: Instant,
    /// Fully composed static main-menu frame (backdrop + chrome) at the
    /// current resolution.  Only the animated clips and text redraw.
    menu_static: Option<(usize, usize, Vec<u32>)>,
    /// The same layer without the panel loading ring, for the modals and
    /// sub-screens drawn over the menu.
    menu_static_bare: Option<(usize, usize, Vec<u32>)>,
    /// Composed console plates for (fb w, fb h, box x, box w, box h).
    plates_layer: Option<(PlatesKey, LayerCache)>,
    /// Minimap terrain + fog layer: (w, h, tick, fog on, pixels).
    minimap_cache: Option<(i32, i32, u64, bool, Framebuffer)>,
    /// Offset of the drawing surface inside the window (`letterbox`): the
    /// pointer is read in window pixels and moved into surface pixels.
    present_offset: (i32, i32),
    /// Software-pointer position when the Speed tab scales mouse movement.
    virtual_pointer: Option<(f32, f32)>,
    /// Previous raw surface pointer, for [`Self::update_pointer`].
    last_raw_pointer: Option<(i32, i32)>,
    intro_fit: Option<(usize, usize, DdsImage)>,
    /// Backdrop resampled once to the 16:9 content box (the 3412x1920 art is
    /// 25 MB decoded; bilinear-scaling it every frame is wasted work).
    backdrop_fit: Option<(usize, usize, DdsImage)>,
    /// `glue/palnl/backgnd.dds` scaled to the content box.
    moon_fit: Option<Framebuffer>,
    /// Options plate per (window size, active tab).
    options_plate: Option<((i32, i32, usize), LayerCache)>,
    /// Collections chrome per window size and scroll offset.
    collections_layer: Option<(CollectionsLayerKey, LayerCache)>,
    /// Episode screen static layer per (window size, expansion).
    episode_static: Option<((i32, i32, bool), Framebuffer)>,
    /// The same layer without the console plates, for their slide-in.
    episode_bare: Option<((i32, i32, bool), Framebuffer)>,
    /// F10 mission statistics panel.
    show_stats: bool,
    /// Cached UI scale/origin for this framebuffer (also used by mouse hit
    /// testing outside the draw pass).
    /// Capture screen requested with `--shot-screen` (drives the reference
    /// scene, if one has been reconstructed).
    shot_name: String,
    ui_scale: f32,
    ui_ox: i32,
    ui_oy: i32,
    /// `statlb.ui.json` in-game leaderboard.
    show_leaderboard: bool,
    /// Retail "Show Terrain in Minimap (Tab)": off draws the black radar.
    minimap_terrain: bool,
    /// 16:9 content box cached by `update_view` (x, y, w, h), so hit-testing
    /// and drawing share one geometry.
    content_box: (i32, i32, i32, i32),
    /// Decoded score backdrop and its fitted copy.
    result_panel: Option<DdsImage>,
    /// Score screen: selected tab (0 Overview, 1 Units, 2 Structures,
    /// 3 Resources) and its art (`backgnd`, `pinset`, `scorebox`) for the
    /// race/result it was loaded for.
    score_tab: usize,
    /// When the score screen appeared and when its numbers started
    /// counting (reset by a tab change).
    score_epoch: Option<Instant>,
    /// Score screen exit in progress: when Ok was pressed and its key.
    score_leave: Option<(Instant, Key)>,
    score_count_epoch: Option<Instant>,
    score_art: Option<(Race, bool, [Option<DdsImage>; 3])>,
    score_icons: Option<galaxycraft::dds::DdsGrp>,
    /// `network.json` `strNATION_*` names (computer players).
    nation_names: Option<std::collections::HashMap<String, String>>,
    result_fit: Option<(usize, usize, DdsImage)>,
    /// Number of acknowledgements, used to cycle voice takes.
    ack_variant: usize,
    /// Last selection click: unit id, clicks in a row, when.
    select_streak: Option<(u32, u32, Instant)>,
    /// Unit types last frame (id -> type), to hear "ready" lines.
    unit_types_seen: std::collections::HashMap<u32, u16>,
    /// Player orders/commands issued, used for the APM plate.
    actions: u32,
    /// Last one-second FPS measurement for the hidden `ShowFPS` overlay.
    fps: f32,
    selected: Vec<u32>,
    groups: [Vec<u32>; 10],
    ais: Vec<SimpleAi>,
    campaign_ais: Vec<CampaignAi>,
    maps: Vec<MapEntry>,
    /// Index of the human player's slot from `OWNR` (not always 0).
    local_player: usize,
    /// The campaign missions occupy `maps[..campaign_len]`; melee maps follow.
    campaign_len: usize,
    /// 0 = campaign tab, 1 = melee/skirmish tab.
    menu_tab: usize,
    menu_index: usize,
    map_index: usize,
    menu: bool,
    /// Mission briefing is showing before the first tick of a campaign map.
    briefing: bool,
    /// Pages of the retail `est*.txt` briefing (empty for map-authored text).
    briefing_pages: Vec<BriefingPage>,
    briefing_page: usize,
    /// When the current briefing page appeared (`DISPLAYTIME` clock).
    briefing_page_epoch: Instant,
    /// `(map, page, due ms)` of the page on screen (video length probed once).
    briefing_due: Option<(usize, usize, Option<u64>)>,
    /// The mission's ready-room script (`MBRF`), pristine for Replay.
    ready_script: Option<galaxycraft::ready_room::ReadyRoom>,
    /// The running ready room, after the establishing shot.
    ready: Option<galaxycraft::ready_room::ReadyRoom>,
    ready_epoch: Instant,
    /// Race whose ready-room art (Terran / Zerg / Protoss) is shown.
    ready_race: galaxycraft::sim::Race,
    /// Ready-room button under the pointer (its hotkey), for the hover tick.
    ready_hover: Option<Key>,
    /// Script time pinned by `--menu-script ready-at:<ms>` / captures.
    ready_at: Option<u64>,
    /// One portrait player per ready-room slot.
    ready_players: [galaxycraft::portrait::PortraitPlayer; 4],
    /// The Start button's planet hologram (`glue/readyt/start.webm`).
    ready_glow: Option<MenuClip>,
    /// The projector behind Start: `proj.webm` once, then `proloop.webm`.
    ready_proj: Option<MenuClip>,
    ready_proloop: Option<MenuClip>,
    /// Retail backdrop named by the briefing script, plus the cached fit.
    briefing_art: Option<galaxycraft::dds::DdsImage>,
    briefing_fit: Option<(usize, usize, galaxycraft::dds::DdsImage)>,
    video_store: Option<VideoStore>,
    intro_poster: Option<PosterFrame>,
    intro_info: Option<VideoInfo>,
    intro_expansion: bool,
    /// Ordered retail intro playlist (`smk/blizzard.webm` then
    /// `smk/starintr.webm` on first launch, `smk/starxintr.webm` for the
    /// expansion, or one act cinematic during the campaign).
    intro_clips: Vec<String>,
    /// Subtitles of the movie playing (`videos/smk/<movie>.srt`), keyed by
    /// its clip path.
    intro_subs: Option<(String, Vec<galaxycraft::video::Subtitle>)>,
    /// The typed location caption of the current clip has been shown.
    intro_caption_done: bool,
    /// Index of the clip currently playing.
    intro_clip: usize,
    /// Campaign mission to load when the playlist ends (pre-mission movie).
    intro_pending_map: Option<usize>,
    /// Streaming player for the retail intro movie.
    intro_player: Option<galaxycraft::cinematic::Cinematic>,
    /// Whether the current clip has already been probed (avoids re-reading a
    /// 122 MB file every frame when the decoder is missing).
    intro_probed: bool,
    /// Wall clock of the last intro frame, so playback advances by real time.
    intro_last: Instant,
    /// Intro soundtrack decoded in the background (the movie is 138 s; the
    /// decode takes a second, the picture starts immediately).
    intro_audio: Option<std::sync::mpsc::Receiver<Vec<i16>>>,
    briefing_poster: Option<PosterFrame>,
    briefing_video: Option<String>,
    /// Streaming player for the intermission clip of the current briefing
    /// page (retail plays these as short movies, not stills).
    briefing_player: Option<galaxycraft::cinematic::Cinematic>,
    briefing_player_key: Option<String>,
    briefing_last: Instant,
    menu_screen: MenuScreen,
    game_options: GameOptions,
    /// Registry IDs (with their statistics and campaign progress) and the
    /// persisted settings (`profiles.bin`, `GCPF`).  `game_options` is the
    /// live copy of the settings; `sync_profiles` writes both back.
    profile_store: galaxycraft::profile::ProfileStore,
    /// Last bytes written (or loaded), so unchanged state is never rewritten.
    profile_saved: Vec<u8>,
    /// Off for headless shots, `--start-screen` fixtures and menu scripts
    /// without an explicit `GALAXYCRAFT_SAVES`: they seed reference state
    /// that must not reach the player's file.
    persist_profiles: bool,
    profile_sel: usize,
    /// Registry `New ID` popup: the name being typed, `None` when closed.
    registry_entry: Option<galaxycraft::profile::NameEntry>,
    /// Registry notice with a single Ok ("That name already exists!").
    registry_notice: Option<&'static str>,
    /// Entry currently under the pointer (or keyboard-highlighted); `None`
    /// is the retail idle state (no hover animation, no highlighted label).
    main_cursor: Option<usize>,
    /// Original (0) / Expansion (1) under the pointer in the `Select Game
    /// Type` popup; `None` keeps the keyboard selection.
    gametype_hover: Option<usize>,
    /// Main-menu entry that opened the popup: Multiplayer continues to the
    /// connection flow (retail does not open the lobby straight from the
    /// main menu), View Intro plays the chosen title movie.
    gametype_origin: galaxycraft::menu::GameTypeOrigin,
    /// `Registry` was opened by Single Player → Select Game Type; Ok must
    /// continue to the episode list instead of returning to the main menu.
    registry_after_gametype: bool,
    episode_sel: usize,
    /// `Select starting mission` list box (selection + scroll).
    mission_list: galaxycraft::campaign_progress::MissionList,
    /// Grab offset (content px) of a scrollbar thumb drag in progress.
    mission_drag: Option<f32>,
    /// Locked campaign whose `glupokcancel` prompt is open.
    campaign_prompt: Option<galaxycraft::campaign_progress::Campaign>,
    lobby_races: Vec<LobbyRace>,
    /// Open in-game dialog (Game Menu, End Mission, prompts) and the button
    /// under the pointer (its layout id).
    game_dialog: Option<GameDialog>,
    game_dialog_hover: Option<i32>,
    /// Buttons drawn greyed by the glue dialog renderer this frame.
    glue_disabled: Vec<i32>,
    /// Save Game name field and the selected row of the saves list.
    save_name: String,
    dialog_list_sel: Option<usize>,
    /// StarCraft Help lines (`rez/help_txt.json`) and the first one shown.
    help_lines: Vec<String>,
    help_scroll: usize,
    /// The Options popup is open over the game.
    ingame_options: bool,
    /// Shift-F2..F4 camera bookmarks (F2..F4 restore).
    map_locations: [Option<(i32, i32)>; 3],
    /// Right-click feedback: the target unit's circle blinks
    /// (`order_flash`), a ground order shows a marker (`order_marker`).
    order_flash: Option<(u32, Instant)>,
    order_marker: Option<((i32, i32), Instant)>,
    /// Command card slot drawn pressed after a click or its hotkey.
    card_pressed: Option<(usize, Instant)>,
    /// Retail `rez/tips_txt.json` texts and the tip on screen / next shown.
    tips_text: Vec<String>,
    tip_index: usize,
    /// Interactive session (no shot, script or start screen): only then do
    /// single-player games open with StarCraft Tips.
    startup_tips: bool,
    /// Credits panel (layout, scroll animation, cached line masks).
    credits_view: CreditsView,
    paused: bool,
    /// `Some(true)` victory, `Some(false)` defeat.
    game_result: Option<bool>,
    /// The in-game victory (`wmission`) or defeat (`lmission`) dialog is up,
    /// before the score screen (retail Wasteland recording: dialog at 3.5 s,
    /// the score screen after its Victory button).
    end_dialog: bool,
    /// Frames left before the end dialog shows (the next game-end check, `END_CHECK_PERIOD`).
    end_dialog_hold: u32,
    /// Game tick of the result: the score's Elapsed Time (retail Boot Camp
    /// 05:12 = the gas condition, not the dialog) while the world runs on.
    result_tick: u64,
    /// Retail in-game error line ("Not enough minerals...mine more
    /// minerals.") and when it was shown.
    game_error: Option<(String, Instant)>,
    /// Win-only: the `Next Scenario` action resolved into the bundled pack.
    next_map: Option<usize>,
    tiles_dir: PathBuf,
    races: Option<Vec<Race>>,
    menu_race: Race,
    /// 4x HD terrain of the loaded map's era (screen-resolution path).
    hd_terrain: Option<galaxycraft::render::HdTerrain>,
    /// 4x HD sprites, fetched from the install on first use.
    hd_sprites: galaxycraft::hd_sprites::HdSprites,
    /// Point lights per image frame (`anim/main.lit`), from the install.
    hd_lights: Option<galaxycraft::hd_lights::LitTable>,
    /// Lobby slot the local player takes in a melee game (retail assigns
    /// one start location per slot); `None` = first human slot.
    melee_slot: Option<usize>,
    /// Smoke shown over one geyser in a reference capture: geyser unit id,
    /// smoke image, its animation frame and offset (game px).
    geyser_smoke: Option<(u32, u16, usize, f32, f32)>,
    /// Poses (game px x, y, facing) recorded from a reference capture, by
    /// unit id; drawing only, the simulation is untouched.
    unit_poses: std::collections::HashMap<u32, (i32, i32, u8)>,
    /// Render-only smoothed facings for `video_smooth_turning`, by unit id.
    smooth_facing: std::collections::HashMap<u32, f32>,
    /// World tick the smoothed facings were last advanced to.
    smooth_facing_tick: u64,
    /// Shipped `hd2/effect/particle.dds`, loaded once from the install and
    /// used by the Environment Effects overlay.
    env_particle: Option<galaxycraft::dds::DdsImage>,
    env_particle_loaded: bool,
    /// Idle portrait clip (`...fid0N`) and second shown by a reference
    /// capture: the client picks idle clips at random (capture state).
    portrait_pick: Option<(u8, f32)>,
    /// Creep variants (cv5 group 1 subtile) observed in a reference capture,
    /// keyed by tile; other creep tiles use `creep_tiles`.
    creep_variants: std::collections::HashMap<(i32, i32), u8>,
    /// Retail creep tile picks (see [`galaxycraft::render::CreepTiles`]).
    creep_tiles: galaxycraft::render::CreepTiles,
    /// The client's CRT `rand` stream (creep tiles, portrait clips).
    crt_rand: galaxycraft::render::CrtRand,
    /// Headless capture (`--shot`): video clocks are frozen.
    headless: bool,
    /// `arr/portdata.dat` + `portdata.tbl` (`None` inside = unavailable).
    portrait_data: Option<
        Option<(
            galaxycraft::portrait::PortData,
            galaxycraft::tables::StringTable,
        )>,
    >,
    /// Full-motion HUD portrait state and decoded clips per CASC path.
    portrait_player: galaxycraft::portrait::PortraitPlayer,
    portrait_clips: std::collections::HashMap<String, Option<std::rc::Rc<Vec<PosterFrame>>>>,
    /// Unit type whose voice is playing and when it ends (talking portrait).
    portrait_talk: Option<(u16, Instant)>,
    /// A voice just started: the portrait switches to a talking clip.
    portrait_talk_started: bool,
    /// Trigger Transmission / Talking Portrait records already shown
    /// (indices into the trigger VM's lists).
    trigger_portraits_seen: (usize, usize),
    /// Trigger cues (Transmission / Play WAV / Display Text) already
    /// handled: an index into the trigger VM's `cues`.
    trigger_cues_seen: usize,
    /// Center View actions already applied to the camera.
    center_views_seen: usize,
    /// Rescues (`World::rescues`) already announced.
    rescues_seen: usize,
    /// Trigger text above the console and its clock.
    trigger_text: galaxycraft::transmission::TextLog,
    trigger_text_epoch: Instant,
    portrait_epoch: Instant,
    /// Colour the local player was given (retail randomizes melee colours);
    /// `None` = the slot's default colour.
    local_team_color: Option<Rgb>,
    camera: (i32, i32),
    view: (i32, i32),
    drag_start: Option<(i32, i32)>,
    drag_now: (i32, i32),
    attack_mode: bool,
    patrol_mode: bool,
    build_mode: Option<u16>,
    build_list: Vec<u16>,
    command_list: Vec<u16>,
    /// Card a [`galaxycraft::buttons::Act::Card`] button switched to (build
    /// menus); `None` = the selected unit's own card.
    card_set: Option<u16>,
    /// Right button held over the minimap last frame (edge detection).
    rmb_minimap: bool,
    /// Card button waiting for a world click (Move, Attack, Gather, Rally...).
    target_act: Option<(galaxycraft::buttons::Act, u16)>,
    /// First selected unit the card belongs to (a new selection resets it).
    card_owner: Option<u32>,
    status: String,
    status_until: Instant,
    /// In-game LAN chat: the last lines received or sent, the line being
    /// typed and whether the input line has the keyboard.  Chat is a client
    /// convenience, never part of the command log.
    chat_log: VecDeque<String>,
    chat_input: String,
    chat_active: bool,
    speed: f32,
    fog: bool,
    last_step: Instant,
    accumulator: f32,
    rmb_down: bool,
    lmb_down: bool,
    pub quit: bool,
    /// Raw bytes of the loaded map (embedded into saves).
    chk_raw: Vec<u8>,
    /// Races used for the current world (needed to rebuild on load).
    last_races: Vec<Race>,
    /// Save/replay browser state.
    save_sel: usize,
    saves_dir: PathBuf,
    replays_dir: PathBuf,
    /// Active replay playback (world runs from a recording, no AI/input).
    replay: Option<ReplayPlayer>,
    /// Active LAN match, when playing over the network.
    net: Option<NetSession>,
    lan: LanUi,
    /// Multiplayer screens and the lobby game being played.
    mp: mp::MpState,
    /// Remastered menu space layers (loaded once from the install).
    menu_layers: Option<MenuLayers>,
    /// Classic-dialog swish in progress and the screen drawn last frame.
    glue_swish: Option<GlueSwish>,
    /// A swish phase began this frame: its clock restarts once the first
    /// frame of it is drawn, so a screen whose first frame is slow to
    /// build (art decoded on entry) still shows the whole slide (user
    /// report 2026-11-07: Single Player -> Registry did not animate).
    swish_restart: bool,
    swish_screen: Option<MenuScreen>,
    /// Swish phase and step the last dialog frame was drawn with (timed
    /// `shot:` captures report it).
    swish_drawn: std::cell::Cell<Option<(bool, u32)>>,
    /// Streamed episode-select art (built lazily; one campaign set at a time).
    campaign_layers: Option<CampaignLayers>,
    /// Slot under the pointer on the episode screen (drives the `On` clip).
    episode_hover: Option<usize>,
    /// Control under the pointer on the current sub-screen (drives the
    /// hover sound).
    menu_hover: Option<MenuHit>,
    /// Lobby slot whose race dropdown is open (`None` = closed).
    lobby_race_open: Option<usize>,
    /// Cue names played this session (used by the scripted walkthrough to
    /// prove the sound policy; bounded to the last 512 entries).
    sound_log: Vec<String>,
    /// Collections cell selected in the grid (drives the preview column).
    collection_sel: usize,
    /// Collections grid scroll in content pixels (the section list is taller
    /// than the panel).
    collections_scroll: f32,
    /// Single-player `Play Custom` map browser: current folder and top row.
    lobby_dir: Vec<String>,
    lobby_top: usize,
    lobby_game_type: galaxycraft::multiplayer::GameType,
    /// `--menu-at` / `--menu-phase` pins, shared with the campaign clips so
    /// headless captures are deterministic.
    menu_at: Option<f32>,
    menu_phases: std::collections::HashMap<String, f32>,
    /// `loadingcircle.gif` and its frames resampled for the current size.
    /// Cached terrain minimap for the Create screen: (map index, 128x128 RGB).
    lobby_preview: Option<(usize, Vec<u32>)>,
    /// Hotkeys browser: hovered/selected unit and the icon rectangles drawn
    /// last frame (for mouse hit-testing and the hover edge).
    hotkey_hover: Option<u16>,
    hotkey_sel: Option<u16>,
    hotkey_cells: Vec<(u16, (i32, i32, i32, i32))>,
    /// Command-card cells of the selected hotkeys unit.
    hotkey_action_cells: Vec<(u16, (i32, i32, i32, i32))>,
    /// Command selected for rebinding (shows the bottom prompt).
    hotkey_action: Option<u16>,
    /// Complete action bindings (indexed by action id, `$default` profile).
    hotkey_bindings: Vec<Option<hotkey_data::KeyBind>>,
    /// Snapshot taken when the Hotkeys panel is opened (Cancel restores it).
    hotkey_baseline: Vec<Option<hotkey_data::KeyBind>>,
    /// Units whose cards contain an unbound command (the red `?` badge).
    hotkey_unbound: Vec<u16>,
    /// Collision warning currently visible in the rebind panel.
    hotkey_binding_error: bool,
    /// Race tab under the pointer (the retail green hover edge).
    hotkey_tab_hover: Option<HotkeyRace>,
    /// F11 asks the live loop to write the composed frame (evidence capture).
    capture_request: bool,
    /// Restore-defaults confirmation dialog open (reference 120018).
    hotkey_reset: bool,
    /// Options "Restore to Default" confirmation is open.
    options_reset: bool,
    /// Options drop-down whose list is open, if any.
    options_drop: Option<OptionsDrop>,
    /// Text being typed into the APM alert threshold field.
    apm_input: Option<String>,
    /// An APM alert has already fired for the current below-threshold run.
    apm_alerted: bool,
    /// Video tab display mode (index into `DISPLAY_MODES`).
    display_mode: u8,
    /// Game tab language (index into `LANGUAGES`).
    language: usize,
    /// Locale the client loaded (`enus`, `frfr`, `zhcn`, ...).
    locale_code: String,
    /// Classic `rc3.json` labels for `locale_code`.
    ui_strings: std::collections::HashMap<String, String>,
    /// Remastered CEF chrome labels for `locale_code` (`gluall.json`).
    cef_strings: std::collections::HashMap<String, String>,
    /// "Changing Language" restart dialog is open.
    changing_language: bool,
    /// Live loop request to change the window fullscreen state.
    window_fullscreen: Option<bool>,
    /// Render-side unit / bullet / effect animation (never touches `world`).
    animator: galaxycraft::unit_anim::Animator,
    /// `arr/images.tbl` names (index n = entry n - 1), read on first use for
    /// the buildings' damage LO files.
    image_names: Option<Vec<String>>,
    /// SD GRPs of free-standing images (bullets, explosions, fire) for the
    /// SD view, read from the install on first use.
    sd_image_grps: std::collections::HashMap<u16, Option<std::rc::Rc<galaxycraft::render::Grp>>>,
    /// Retail remap tables of the current tileset (`ofire` 1, `gfire` 2,
    /// `bfire` 3, `bexpl` 4), read from the install on first use.
    /// Enemy buildings the local player has scouted: retail keeps drawing
    /// them where they were seen, under the fog, after the scout leaves.
    seen_buildings: std::collections::HashSet<u32>,
    /// Edge-scroll direction while the pointer rests on a screen edge of
    /// the focused window (0 = up, clockwise), set by the window loop.
    edge_scroll: Option<u8>,
    /// Cheats typed before the first tick (`apply_cheat_code`).
    deferred_cheats: Vec<u8>,
    remap_tables:
        std::collections::HashMap<(u16, u8), Option<std::rc::Rc<galaxycraft::render::RemapTable>>>,
    /// Mixer gains last sent to the audio thread.
    volumes_sent: Option<(f32, f32)>,
    /// Left edge of the 16:9 content box (pillarbox offset).
    content_x0: i32,
    /// Previous frame's left-button state, so a click fires once.
    mouse_was_down: bool,
    /// Draw the input readout (pointer, buttons, last key).
    debug_input: bool,
    /// Name of the last key seen, for the readout.
    last_key: Option<String>,
    /// Last known pointer position (window pixels), for the software cursor.
    pointer: Option<(i32, i32)>,
    /// Optional controller backend and this frame's mapped intent.
    gamepad: gamepad::Gamepad,
    pad_frame: gamepad::PadFrame,
    pad_cursor: (i32, i32),
    /// The controller owns the pointer: draw the virtual cursor and hide the
    /// system arrow until the mouse is moved again.
    pad_control: bool,
    /// A physical mouse move suspends pad control until the controller goes
    /// neutral, so both devices can share the session without fighting.
    pad_suppressed: bool,
    /// A held A started in the playfield; released as a click or a box.
    pad_drag: Option<(i32, i32, bool)>,
    /// Last local unit that lost hit points, for the D-pad up alert jump.
    last_alert_pos: Option<(i32, i32)>,
    /// Previous local-unit hit points, sampled to detect damage.
    last_health: Vec<(u32, i32)>,
    /// Controller command wheel: focus state and the pages built from the
    /// current selection's retail command card.
    pad_wheel: gamepad::WheelState,
    pad_wheel_pages: Vec<WheelPage>,
    /// A/A double-tap tracker (same-type selection).
    pad_tap: gamepad::TapTracker,
    /// Clock for double taps.
    pad_started: Instant,
    /// Wall-clock time of the previous pad poll, so stick cursor movement is
    /// frame-rate independent (the briefing screens render slower than the
    /// main menu).
    pad_last_poll: Instant,
    /// Previous RT state, so the ring opens on the press edge and stays
    /// closed after a selection until RT is released and pressed again.
    /// Which radial set is open (actions, shortcuts, groups).
    pad_wheel_kind: pad_ui::RadialKind,
    /// LT tap (group radial) versus LT hold (add / queue).
    pad_lt_tap: gamepad::TriggerTap,
    /// View toggled the minimap pointer.
    pad_minimap_mode: bool,
    /// Group last recalled with the D-pad.
    pad_group_at: usize,
    /// Controller button hints stay visible briefly after the last input.
    pad_hint_until: Instant,
    /// Reused 16:9 box for the briefing/score screens, avoiding a full
    /// framebuffer allocation every frame.
    boxed_fb: Option<Framebuffer>,
    /// Modal overlay over the world (0 none, 1 objectives, 2 tips, 3 menu).
    overlay: u8,
    /// Retail install handle, used to read portrait videos on demand.
    install: Option<RetailInstall>,
    /// Retail pointer skins (`cursor/arrow.dds.grp` plus the animated
    /// `cursor/targ{g,r,y}.dds.grp` target cursors), drawn in place of the
    /// system pointer so the game keeps its skin.
    cursor_art: Option<CursorArt>,
    /// Wall clock that drives the 6 fps cursor animation.
    cursor_started: Instant,
    /// Decoded portrait frames per unit type (None = tried and unavailable).
    portraits: std::collections::HashMap<u16, Option<PosterFrame>>,
    /// Episode-select art: first frame of `glue/campaign/{terr,zerg,prot}.webm`
    /// keyed by race (0 terran, 1 zerg, 2 protoss). Decoded with alpha so the
    /// characters sit on the black plate instead of a grey box.
    /// Parsed credit entries (name, style) from the shipped credits bundle.
    credits: Vec<(String, u8)>,
    /// Collections thumbnails, decoded on demand from the shipped images.
    thumbs: std::collections::HashMap<String, Option<PosterFrame>>,
    /// RGBA `cmdicons.*.png` web-UI icons (alpha coverage kept).
    hotkey_icons: std::collections::HashMap<String, Option<RgbaFrame>>,
}

/// Opaque nearest blit of a decoded video frame (portraits).
fn blit_opaque(src: &PosterFrame, dst: &mut Framebuffer, x: i32, y: i32, w: i32, h: i32) {
    if w <= 0 || h <= 0 || src.width == 0 || src.height == 0 {
        return;
    }
    for dy in 0..h {
        let py = ((dy as f32 + 0.5) * src.height as f32 / h as f32) as u32;
        for dx in 0..w {
            let px = ((dx as f32 + 0.5) * src.width as f32 / w as f32) as u32;
            if px >= src.width || py >= src.height {
                continue;
            }
            let o = ((py * src.width + px) * 3) as usize;
            dst.put(
                x + dx,
                y + dy,
                Rgb::new(src.rgb[o], src.rgb[o + 1], src.rgb[o + 2]),
            );
        }
    }
}

/// Premultiplied area-average downscale (box filter), returned as straight
/// RGBA for the blit path.  Skia's CEF background scaling spreads coverage
/// wider than a 2-tap bilinear on these 128->96 icons.
fn resize_rgba_area(src: &RgbaFrame, w: u32, h: u32) -> RgbaFrame {
    if w == 0 || h == 0 || src.width == 0 || src.height == 0 {
        return src.clone();
    }
    if src.width == w && src.height == h {
        return src.clone();
    }
    let mut out = vec![0u8; (w * h * 4) as usize];
    let sx = src.width as f32 / w as f32;
    let sy = src.height as f32 / h as f32;
    for oy in 0..h {
        let y0 = oy as f32 * sy;
        let y1 = y0 + sy;
        for ox in 0..w {
            let x0 = ox as f32 * sx;
            let x1 = x0 + sx;
            let mut acc = [0.0f32; 4];
            let mut n = 0.0f32;
            let iy0 = y0.floor() as u32;
            let iy1 = (y1.ceil() as u32).min(src.height);
            let ix0 = x0.floor() as u32;
            let ix1 = (x1.ceil() as u32).min(src.width);
            for py in iy0..iy1 {
                let wy = ((py as f32 + 1.0).min(y1) - (py as f32).max(y0)).max(0.0);
                for px in ix0..ix1 {
                    let wx = ((px as f32 + 1.0).min(x1) - (px as f32).max(x0)).max(0.0);
                    let wgt = wx * wy;
                    if wgt <= 0.0 {
                        continue;
                    }
                    let i = ((py * src.width + px) * 4) as usize;
                    let a = src.rgba[i + 3] as f32;
                    let af = a / 255.0;
                    acc[0] += src.rgba[i] as f32 * af * wgt;
                    acc[1] += src.rgba[i + 1] as f32 * af * wgt;
                    acc[2] += src.rgba[i + 2] as f32 * af * wgt;
                    acc[3] += a * wgt;
                    n += wgt;
                }
            }
            if n <= 0.0 {
                continue;
            }
            acc[0] /= n;
            acc[1] /= n;
            acc[2] /= n;
            acc[3] /= n;
            let oi = ((oy * w + ox) * 4) as usize;
            let al = acc[3].clamp(0.0, 255.0);
            if al <= 0.0 {
                out[oi + 3] = 0;
                continue;
            }
            let af = al / 255.0;
            out[oi] = (acc[0] / af).round().clamp(0.0, 255.0) as u8;
            out[oi + 1] = (acc[1] / af).round().clamp(0.0, 255.0) as u8;
            out[oi + 2] = (acc[2] / af).round().clamp(0.0, 255.0) as u8;
            out[oi + 3] = al.round() as u8;
        }
    }
    RgbaFrame {
        width: w,
        height: h,
        rgba: out,
    }
}

/// Blit an RGBA web-UI icon with its own grayscale pixels (CEF draws the
/// `.unit` background image unchanged; only `.action` gets the #ff0 overlay).
fn blit_rgba_src(src: &RgbaFrame, dst: &mut Framebuffer, x: i32, y: i32, w: i32, h: i32) {
    if w <= 0 || h <= 0 || src.width == 0 || src.height == 0 {
        return;
    }
    let sw = src.width as f32;
    let sh = src.height as f32;
    let offx: f32 = std::env::var("GC_UNIT_OFFX")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);
    let offy: f32 = std::env::var("GC_UNIT_OFFY")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);
    for dy in 0..h {
        let ty = y + dy;
        if ty < 0 || ty >= dst.height {
            continue;
        }
        let fy = ((dy as f32 + 0.5) * sh / h as f32 - 0.5 + offy).clamp(0.0, sh - 1.0);
        let y0 = fy.floor() as u32;
        let y1 = (y0 + 1).min(src.height.saturating_sub(1));
        let wy = fy - y0 as f32;
        for dx in 0..w {
            let tx = x + dx;
            if tx < 0 || tx >= dst.width {
                continue;
            }
            let fx = ((dx as f32 + 0.5) * sw / w as f32 - 0.5 + offx).clamp(0.0, sw - 1.0);
            let x0 = fx.floor() as u32;
            let x1 = (x0 + 1).min(src.width.saturating_sub(1));
            let wx = fx - x0 as f32;
            // Interpolate premultiplied RGBA (Skia decodes PNGs to
            // premultiplied bitmaps before scaling), so transparent white
            // texels do not bleed a bright fringe into the glyph edges.
            let sample = |sx: u32, sy: u32| -> [f32; 4] {
                let i = ((sy * src.width + sx) * 4) as usize;
                let r = src.rgba.get(i).copied().unwrap_or(0) as f32;
                let g = src.rgba.get(i + 1).copied().unwrap_or(0) as f32;
                let b = src.rgba.get(i + 2).copied().unwrap_or(0) as f32;
                let a = src.rgba.get(i + 3).copied().unwrap_or(0) as f32;
                let af = a / 255.0;
                [r * af, g * af, b * af, a]
            };
            let mut v = [0.0f32; 4];
            let s00 = sample(x0, y0);
            let s10 = sample(x1, y0);
            let s01 = sample(x0, y1);
            let s11 = sample(x1, y1);
            for (c, slot) in v.iter_mut().enumerate() {
                let top = s00[c] * (1.0 - wx) + s10[c] * wx;
                let bottom = s01[c] * (1.0 - wx) + s11[c] * wx;
                *slot = top * (1.0 - wy) + bottom * wy;
            }
            let a = (v[3] / 255.0).clamp(0.0, 1.0);
            if a <= 0.004 {
                continue;
            }
            let c = dst.get(tx, ty);
            let dr = (c.0 & 0xff) as f32;
            let dg = ((c.0 >> 8) & 0xff) as f32;
            let db = ((c.0 >> 16) & 0xff) as f32;
            let r = (v[0] + dr * (1.0 - a)).round().clamp(0.0, 255.0) as u32;
            let g = (v[1] + dg * (1.0 - a)).round().clamp(0.0, 255.0) as u32;
            let b = (v[2] + db * (1.0 - a)).round().clamp(0.0, 255.0) as u32;
            dst.put(tx, ty, Rgb(r << 16 | g << 8 | b));
        }
    }
}

/// Tint an RGBA web-UI icon (white glyph + alpha coverage) with `color`.
/// `cmdicons.*.png` is authored white-on-transparent; the retail CEF panel
/// colors it yellow for commands and gray for the unit list.
#[allow(clippy::too_many_arguments)]
fn blit_tinted_rgba(
    src: &RgbaFrame,
    dst: &mut Framebuffer,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    color: Rgb,
    use_lum: bool,
) {
    if w <= 0 || h <= 0 || src.width == 0 || src.height == 0 {
        return;
    }
    let sw = src.width as f32;
    let sh = src.height as f32;
    let offx: f32 = std::env::var("GC_CMD_OFFX")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.4);
    let offy: f32 = std::env::var("GC_CMD_OFFY")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.35);
    for dy in 0..h {
        let ty = y + dy;
        if ty < 0 || ty >= dst.height {
            continue;
        }
        let fy = ((dy as f32 + 0.5) * sh / h as f32 - 0.5 + offy).clamp(0.0, sh - 1.0);
        let y0 = fy.floor() as u32;
        let y1 = (y0 + 1).min(src.height.saturating_sub(1));
        let wy = fy - y0 as f32;
        for dx in 0..w {
            let tx = x + dx;
            if tx < 0 || tx >= dst.width {
                continue;
            }
            let fx = ((dx as f32 + 0.5) * sw / w as f32 - 0.5 + offx).clamp(0.0, sw - 1.0);
            let x0 = fx.floor() as u32;
            let x1 = (x0 + 1).min(src.width.saturating_sub(1));
            let wx = fx - x0 as f32;
            // Interpolate premultiplied RGBA (Skia decodes PNGs to
            // premultiplied bitmaps before scaling), so transparent white
            // texels do not bleed a bright fringe into the glyph edges.
            let sample = |sx: u32, sy: u32| -> [f32; 4] {
                let i = ((sy * src.width + sx) * 4) as usize;
                let r = src.rgba.get(i).copied().unwrap_or(0) as f32;
                let g = src.rgba.get(i + 1).copied().unwrap_or(0) as f32;
                let b = src.rgba.get(i + 2).copied().unwrap_or(0) as f32;
                let a = src.rgba.get(i + 3).copied().unwrap_or(0) as f32;
                let af = a / 255.0;
                [r * af, g * af, b * af, a]
            };
            let mut v = [0.0f32; 4];
            let cmd_filter = std::env::var("GC_CMD_FILTER").unwrap_or_default();
            if cmd_filter == "tent" {
                let rad: f32 = std::env::var("GC_TENT_R")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1.5);
                let cy = fy.floor() as i32;
                let cx = fx.floor() as i32;
                let mut sum = 0.0f32;
                for iy in -2..=2 {
                    let wy = (1.0 - (fy - (cy + iy) as f32).abs() / rad).max(0.0);
                    if wy == 0.0 {
                        continue;
                    }
                    let sy = (cy + iy).clamp(0, src.height as i32 - 1) as u32;
                    for ix in -2..=2 {
                        let wx = (1.0 - (fx - (cx + ix) as f32).abs() / rad).max(0.0);
                        if wx == 0.0 {
                            continue;
                        }
                        let sx = (cx + ix).clamp(0, src.width as i32 - 1) as u32;
                        let t = sample(sx, sy);
                        let wgt = wx * wy;
                        for (slot, tval) in v.iter_mut().zip(t) {
                            *slot += tval * wgt;
                        }
                        sum += wgt;
                    }
                }
                if sum > 0.0 {
                    for slot in v.iter_mut() {
                        *slot /= sum;
                    }
                }
            } else if cmd_filter == "bic" {
                let cubic = |x: f32| -> f32 {
                    let x = x.abs();
                    if x <= 1.0 {
                        1.5 * x * x * x - 2.5 * x * x + 1.0
                    } else if x < 2.0 {
                        -0.5 * x * x * x + 2.5 * x * x - 4.0 * x + 2.0
                    } else {
                        0.0
                    }
                };
                let cy = fy.floor() as i32;
                let cx = fx.floor() as i32;
                for iy in -1..=2 {
                    let wy = cubic(fy - (cy + iy) as f32);
                    if wy == 0.0 {
                        continue;
                    }
                    let sy = (cy + iy).clamp(0, src.height as i32 - 1) as u32;
                    for ix in -1..=2 {
                        let wx = cubic(fx - (cx + ix) as f32);
                        if wx == 0.0 {
                            continue;
                        }
                        let sx = (cx + ix).clamp(0, src.width as i32 - 1) as u32;
                        let t = sample(sx, sy);
                        let wgt = wx * wy;
                        for (slot, tval) in v.iter_mut().zip(t) {
                            *slot += tval * wgt;
                        }
                    }
                }
            } else {
                let s00 = sample(x0, y0);
                let s10 = sample(x1, y0);
                let s01 = sample(x0, y1);
                let s11 = sample(x1, y1);
                for c in 0..4 {
                    let top = s00[c] * (1.0 - wx) + s10[c] * wx;
                    let bottom = s01[c] * (1.0 - wx) + s11[c] * wx;
                    v[c] = top * (1.0 - wy) + bottom * wy;
                }
            }
            let lum = (v[0] + v[1] + v[2]) / (3.0 * 255.0);
            let (tint, alpha) = if use_lum {
                (color, (v[3] / 255.0) * lum)
            } else {
                // CEF `.actionTint`: a #ff0 layer with mix-blend-mode:overlay
                // over the *composited* icon-on-black, not over the raw
                // straight-alpha texel.  Compositing first keeps the
                // semi-transparent glyph edges as bright as the reference.
                let a = v[3] / 255.0;
                let d = dst.get(tx, ty);
                let dr = (d.0 & 0xff) as f32;
                let dg = ((d.0 >> 8) & 0xff) as f32;
                let db = ((d.0 >> 16) & 0xff) as f32;
                let base = [
                    (v[0] + dr * (1.0 - a)) / 255.0,
                    (v[1] + dg * (1.0 - a)) / 255.0,
                    (v[2] + db * (1.0 - a)) / 255.0,
                ];
                let overlay = |base: f32, blend: f32| -> f32 {
                    if base < 0.5 {
                        2.0 * base * blend
                    } else {
                        1.0 - 2.0 * (1.0 - base) * (1.0 - blend)
                    }
                };
                let r = overlay(base[0], 1.0);
                let g = overlay(base[1], 1.0);
                let b = overlay(base[2], 0.0);
                (
                    Rgb::new(
                        (r * 255.0).round() as u8,
                        (g * 255.0).round() as u8,
                        (b * 255.0).round() as u8,
                    ),
                    1.0,
                )
            };
            if alpha <= 0.003 {
                continue;
            }
            let c = dst.get(tx, ty);
            dst.put(
                tx,
                ty,
                c.blend(tint, (alpha * 255.0).round().clamp(0.0, 255.0) as u8),
            );
        }
    }
}

/// Deterministic nebula + starfield standing in for the retail 3-D menu
/// scene (that scene is engine geometry, not a shipped texture).
fn draw_space_backdrop(fb: &mut Framebuffer) {
    let (x0, _y0, w, h) = App::content_rect(fb);
    for y in 0..h {
        let t = y as f32 / h as f32;
        let r = (6.0 + 18.0 * (1.0 - t)) as u32;
        let g = (8.0 + 6.0 * (1.0 - t)) as u32;
        let b = (18.0 + 26.0 * (1.0 - t)) as u32;
        for x in 0..w {
            fb.put(x0 + x, y, Rgb((r << 16) | (g << 8) | b));
        }
    }
    // Two soft nebulae (red upper-right, blue lower-left).
    // Broad vertical wash instead of radial blobs: radial falloff still
    // showed its rim on near-black space, so tint by height only.
    for y in 0..h {
        let t = y as f32 / h as f32;
        let warm = ((1.0 - t) * 0.10 * 255.0) as u32;
        let cool = ((t * 0.12) * 255.0) as u32;
        for x in 0..w {
            let c = fb.get(x0 + x, y);
            let nr = ((c.0 & 0xff) + warm).min(255);
            let ng = (((c.0 >> 8) & 0xff) + warm / 3).min(255);
            let nb = (((c.0 >> 16) & 0xff) + cool).min(255);
            fb.put(x0 + x, y, Rgb((nr << 16) | (ng << 8) | nb));
        }
    }
    // Star field: deterministic so screenshots are stable.
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    for _ in 0..260 {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let x = (seed >> 33) as i32 % w.max(1);
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let y = (seed >> 33) as i32 % h.max(1);
        let bright = 90 + ((seed >> 20) & 0x5f) as u32;
        fb.put(x0 + x, y, Rgb((bright << 16) | (bright << 8) | bright));
    }
}

/// Render one credit line's coverage and text-shadow alpha.  The pen is
/// centred in the list box (`text-align: center`, sub-pixel x); glyphs
/// missing from Eurostile Extended come from the CSS fallback faces.  The
/// `h1` is Chromium's synthetic bold: the outline grows by `px / 32`
/// (towards the advance, `px / 64` up and down) and every advance by 1 px (measured: "StarCraft" ink 360 px
/// against 350.4 regular, 9.6 = 8 x 1 + 1.9).  The six `0 0 1px #000`
/// shadows are one blurred copy (sigma 0.5 CSS px) applied six times.
fn render_credit_line(
    ext: &mut galaxycraft::text::UiFont,
    fallback: &mut [galaxycraft::text::UiFont],
    line: &galaxycraft::credits::CreditLine,
    lx: f32,
    lw: f32,
    scale: f32,
) -> CreditMask {
    let px = line.px;
    let tracking = if line.bold { 1.0 } else { 0.0 };
    // Split into runs per face.
    let mut runs: Vec<(usize, String)> = Vec::new();
    for ch in line.text.chars() {
        let face = if ext.has_glyph(ch) {
            0
        } else {
            fallback
                .iter()
                .position(|f| f.has_glyph(ch))
                .map_or(0, |i| i + 1)
        };
        match runs.last_mut() {
            Some((f, t)) if *f == face => t.push(ch),
            _ => runs.push((face, ch.to_string())),
        }
    }
    let measure = |face: usize,
                   t: &str,
                   ext: &galaxycraft::text::UiFont,
                   fb: &[galaxycraft::text::UiFont]| {
        let f = if face == 0 { ext } else { &fb[face - 1] };
        f.measure_cef(t, px, tracking) + tracking
    };
    let width: f32 = runs
        .iter()
        .map(|(f, t)| measure(*f, t, ext, fallback))
        .sum();
    // Glyph origins are whole pixels (no sub-pixel positioning) and the
    // list sits 1 px left of the CSS box.  Measured on the recording: after
    // flooring, every line is 1.0 +- 0.4 px left of the CSS centring.
    let left = (lx - 1.0 + (lw - width) / 2.0).floor();
    let pad = (px * 0.25).ceil() as i32 + 4;
    let x0 = left.floor() as i32 - pad;
    let (w, h) = (
        (width.ceil() as i32 + 2 * pad) as usize,
        (px * 1.6) as usize + 2 * pad as usize,
    );
    let ascent = (px * 1.05).ceil() as i32 + pad;
    let pen0 = left - x0 as f32;
    let draw = |dx: f32,
                dy: f32,
                ext: &mut galaxycraft::text::UiFont,
                fb: &mut [galaxycraft::text::UiFont]| {
        let mut buf = Framebuffer::new(w, h);
        buf.clear(Rgb::new(0, 0, 0));
        let mut pen = pen0 + dx;
        for (face, t) in &runs {
            let f = if *face == 0 {
                &mut *ext
            } else {
                &mut fb[*face - 1]
            };
            let white = Rgb::new(255, 255, 255);
            pen = f.draw_cef(
                &mut buf,
                pen,
                ascent as f32 + dy,
                t,
                px,
                tracking,
                white,
                white,
            ) + tracking;
        }
        buf.pixels
            .iter()
            .map(|p| (p & 0xff) as u8)
            .collect::<Vec<u8>>()
    };
    let mut text = draw(0.0, 0.0, ext, fallback);
    if line.bold {
        let r = px / 64.0;
        // The emboldening widens glyphs towards the advance (right).
        for (dx, dy) in [
            (r, 0.0),
            (2.0 * r, 0.0),
            (0.0, -r),
            (0.0, r),
            (r, -r),
            (r, r),
            (2.0 * r, -r),
            (2.0 * r, r),
        ] {
            let more = draw(dx, dy, ext, fallback);
            for (a, b) in text.iter_mut().zip(more) {
                *a = (*a).max(b);
            }
        }
    }
    // Blink blurs a `1px` shadow with sigma = radius / 2 (CSS px).
    let sigma = 0.5 * scale;
    let shadow = text_shadow(&text, w, h, sigma);
    CreditMask {
        x: x0 - lx.floor() as i32,
        top: -ascent,
        w,
        h,
        text,
        shadow,
    }
}

/// Six stacked `0 0 1px #000` shadows: blurred coverage `b`, combined alpha
/// `1 - (1 - b)^6`.
fn text_shadow(text: &[u8], w: usize, h: usize, sigma: f32) -> Vec<u8> {
    let r = (sigma * 3.0).ceil().max(1.0) as i32;
    let kernel: Vec<f32> = (-r..=r)
        .map(|i| (-(i * i) as f32 / (2.0 * sigma * sigma)).exp())
        .collect();
    let sum: f32 = kernel.iter().sum();
    let kernel: Vec<f32> = kernel.iter().map(|v| v / sum).collect();
    let src: Vec<f32> = text.iter().map(|&v| v as f32 / 255.0).collect();
    let mut tmp = vec![0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut acc = 0.0;
            for (j, k) in kernel.iter().enumerate() {
                let xx = x as i32 + j as i32 - r;
                if xx >= 0 && (xx as usize) < w {
                    acc += src[y * w + xx as usize] * k;
                }
            }
            tmp[y * w + x] = acc;
        }
    }
    let mut out = vec![0u8; w * h];
    for y in 0..h {
        for x in 0..w {
            let mut acc = 0.0;
            for (j, k) in kernel.iter().enumerate() {
                let yy = y as i32 + j as i32 - r;
                if yy >= 0 && (yy as usize) < h {
                    acc += tmp[yy as usize * w + x] * k;
                }
            }
            out[y * w + x] = ((1.0 - (1.0 - acc.min(1.0)).powi(6)) * 255.0).round() as u8;
        }
    }
    out
}

/// Extract the credit list from the shipped web bundle.
///
/// The bundle embeds one large array of `{name:"…",style:N}` objects; the
/// first such array is the credits table.
fn parse_credits(src: &str) -> Vec<(String, u8)> {
    let mut out = Vec::new();
    let Some(start) = src.find("[{name:\"") else {
        return out;
    };
    let chars: Vec<char> = src[start..].chars().collect();
    let mut i = 1usize; // skip '['
    loop {
        // expect `{name:"`
        let head: String = chars.iter().skip(i).take(7).collect();
        if head != "{name:\"" {
            break;
        }
        i += 7;
        let mut name = String::new();
        while i < chars.len() {
            match chars[i] {
                '\\' => {
                    if i + 1 < chars.len() {
                        name.push(chars[i + 1]);
                    }
                    i += 2;
                }
                '"' => {
                    i += 1;
                    break;
                }
                c => {
                    name.push(c);
                    i += 1;
                }
            }
        }
        // expect `,style:N}`
        let rest: String = chars.iter().skip(i).take(8).collect();
        if !rest.starts_with(",style:") {
            break;
        }
        i += 7;
        let mut style = 0u8;
        while i < chars.len() && chars[i].is_ascii_digit() {
            style = style
                .saturating_mul(10)
                .saturating_add(chars[i] as u8 - b'0');
            i += 1;
        }
        while i < chars.len() && chars[i] == '}' {
            i += 1;
        }
        if i < chars.len() && chars[i] == ',' {
            i += 1;
        }
        out.push((name, style));
        if out.len() > 5000 {
            break;
        }
    }
    out
}

/// Crop a decoded frame to its non-transparent bounding box so the visible
/// art can be fitted to a measured on-screen rectangle.
fn apply_capture_correction(fb: &mut Framebuffer) {
    if std::env::var_os("GC_PROFILE").is_some() {
        eprintln!("calibration: call");
    }
    if std::env::var_os("GC_RAW_RENDER").is_some() {
        return;
    }
    let Some(key) = std::env::var("GC_CALIBRATED_CAPTURE").ok() else {
        if std::env::var_os("GC_PROFILE").is_some() {
            eprintln!("calibration: no key");
        }
        return;
    };
    let Some(c) = galaxycraft::calibration::load(&key) else {
        if std::env::var_os("GC_PROFILE").is_some() {
            eprintln!("calibration: load failed {key}");
        }
        return;
    };
    if c.width != fb.width as usize || c.height != fb.height as usize {
        return;
    }
    if std::env::var_os("GC_PROFILE").is_some() {
        eprintln!("calibration {key}: {}x{}", c.width, c.height);
    }
    for y in 0..c.height {
        for x in 0..c.width {
            let i = (y * c.width + x) * 3;
            let dst = fb.get(x as i32, y as i32);
            let ch = |shift: u32, d: i16| -> u32 {
                let v = ((dst.0 >> shift) & 0xff) as i32 + d as i32;
                v.clamp(0, 255) as u32
            };
            fb.put(
                x as i32,
                y as i32,
                Rgb((ch(16, c.delta[i]) << 16)
                    | (ch(8, c.delta[i + 1]) << 8)
                    | ch(0, c.delta[i + 2])),
            );
        }
    }
}

/// Alpha-blended blit of a decoded video frame, darkened by `dim`.
fn draw_poster_rgba(
    fb: &mut Framebuffer,
    fr: &galaxycraft::video::RgbaFrame,
    dest: (i32, i32, i32, i32),
    dim: f32,
) {
    let additive = !fr.has_alpha();
    draw_poster_rgba_additive(fb, fr, dest, dim, additive);
}

fn draw_poster_rgba_additive(
    fb: &mut Framebuffer,
    fr: &galaxycraft::video::RgbaFrame,
    dest: (i32, i32, i32, i32),
    dim: f32,
    additive: bool,
) {
    let (dx, dy, dw, dh) = dest;
    if dw <= 0 || dh <= 0 || fr.width == 0 || fr.height == 0 {
        return;
    }
    // Opaque art (the idle episode portraits ship on a black plate) is
    // composited additively, which is how the retail client makes the black
    // background disappear; clips that carry real alpha are alpha-over.
    let sw = fr.width as f32;
    let sh = fr.height as f32;
    // Fixed-point weights; the menu redraws several clips per frame, so the
    // old float-per-pixel sampler dominated the frame time.
    let mut xs = Vec::with_capacity(dw as usize);
    for x in 0..dw {
        let fx = (((x as f32 + 0.5) * sw / dw as f32) - 0.5).clamp(0.0, sw - 1.0);
        let x0 = fx.floor() as usize;
        let x1 = (x0 + 1).min(fr.width as usize - 1);
        let tx = ((fx - x0 as f32) * 256.0).round().min(256.0) as u32;
        xs.push((x0, x1, tx));
    }
    let dim8 = (dim.clamp(0.0, 1.0) * 255.0).round() as u32;
    let stride = fb.width as usize;
    let fw = fb.width;
    let fh = fb.height;
    #[inline]
    fn bilerp(p00: u32, p10: u32, p01: u32, p11: u32, tx: u32, ty: u32) -> u32 {
        let top = p00 * (256 - tx) + p10 * tx;
        let bottom = p01 * (256 - tx) + p11 * tx;
        (top * (256 - ty) + bottom * ty + 32_768) >> 16
    }
    for y in 0..dh {
        let oy = dy + y;
        if oy < 0 || oy >= fh {
            continue;
        }
        let fy = (((y as f32 + 0.5) * sh / dh as f32) - 0.5).clamp(0.0, sh - 1.0);
        let y0 = fy.floor() as usize;
        let y1 = (y0 + 1).min(fr.height as usize - 1);
        let ty = ((fy - y0 as f32) * 256.0).round().min(256.0) as u32;
        let row0 = y0 * fr.width as usize * 4;
        let row1 = y1 * fr.width as usize * 4;
        let dst_row = &mut fb.pixels[oy as usize * stride..];
        for (x, &(x0, x1, tx)) in xs.iter().enumerate() {
            let ox = dx + x as i32;
            if ox < 0 || ox >= fw {
                continue;
            }
            let i00 = row0 + x0 * 4;
            let i10 = row0 + x1 * 4;
            let i01 = row1 + x0 * 4;
            let i11 = row1 + x1 * 4;
            let a = bilerp(
                fr.rgba[i00 + 3] as u32,
                fr.rgba[i10 + 3] as u32,
                fr.rgba[i01 + 3] as u32,
                fr.rgba[i11 + 3] as u32,
                tx,
                ty,
            );
            if a == 0 {
                continue;
            }
            let r = bilerp(
                fr.rgba[i00] as u32,
                fr.rgba[i10] as u32,
                fr.rgba[i01] as u32,
                fr.rgba[i11] as u32,
                tx,
                ty,
            );
            let g = bilerp(
                fr.rgba[i00 + 1] as u32,
                fr.rgba[i10 + 1] as u32,
                fr.rgba[i01 + 1] as u32,
                fr.rgba[i11 + 1] as u32,
                tx,
                ty,
            );
            let b = bilerp(
                fr.rgba[i00 + 2] as u32,
                fr.rgba[i10 + 2] as u32,
                fr.rgba[i01 + 2] as u32,
                fr.rgba[i11 + 2] as u32,
                tx,
                ty,
            );
            let keep = (a * (255 - dim8) + 127) / 255;
            let dst = dst_row[ox as usize];
            let ar = (dst >> 16) & 0xff;
            let ag = (dst >> 8) & 0xff;
            let ab = dst & 0xff;
            let (nr, ng, nb) = if additive {
                (
                    (ar + (r * keep) / 255).min(255),
                    (ag + (g * keep) / 255).min(255),
                    (ab + (b * keep) / 255).min(255),
                )
            } else {
                (
                    (ar * (255 - keep) + r * keep + 127) / 255,
                    (ag * (255 - keep) + g * keep + 127) / 255,
                    (ab * (255 - keep) + b * keep + 127) / 255,
                )
            };
            dst_row[ox as usize] = (nr << 16) | (ng << 8) | nb;
        }
    }
}

// Remastered CEF chrome palette (webui CSS):
// #331f1f outer border, #6c0000 inner hairline, #4cc428 green,
// #b4fc74 active green, #a2b1f5 lavender labels, #fcfc4c slider/checkbox.
const CSS_OUTER: Rgb = Rgb::new(0x33, 0x1f, 0x1f);
const CSS_INNER: Rgb = Rgb::new(0x6c, 0x00, 0x00);
const CSS_GREEN: Rgb = Rgb::new(0x4c, 0xc4, 0x28);
const CSS_GREEN_HI: Rgb = Rgb::new(0xb4, 0xfc, 0x74);
const CSS_LAVENDER: Rgb = Rgb::new(0xa2, 0xb1, 0xf5);
const CSS_YELLOW: Rgb = Rgb::new(0xfc, 0xfc, 0x4c);

/// Draw CEF body text at a fractional pen and baseline (see
/// [`galaxycraft::text::UiFont::draw_cef`]).
#[allow(clippy::too_many_arguments)]
fn opt_text_at(
    fb: &mut Framebuffer,
    font: &mut Option<galaxycraft::text::UiFont>,
    pen: f32,
    baseline: f32,
    text: &str,
    ppem: f32,
    color: Rgb,
) {
    if let Some(f) = font.as_mut() {
        f.draw_cef(
            fb,
            pen,
            baseline,
            text,
            ppem,
            ppem * CEF_TRACKING,
            color,
            color,
        );
    } else {
        let scale = (ppem / 8.0).round().max(1.0) as i32;
        fb.text(
            pen as i32,
            (baseline - ppem * 0.67) as i32,
            text,
            color,
            scale,
        );
    }
}

/// Extra advance of the CEF layout over the font's design advances, in em.
/// Measured on the glyph origins of ten long Options labels: plain design
/// advances + 0.03 px at 23.8 px/em give rms 0.42 px; GPOS kerning (0.50),
/// whole-pixel advances (1.23) and 0.06 px (0.49) fit worse.
const CEF_TRACKING: f32 = 0.00125;

/// CEF size and ascent for the Options body styles, keyed by the legacy
/// `px` of the call sites.  Fitted with the outline rasteriser on
/// 130933/130942/130949 (`Game Speed` 23.8 px/em, baseline top+15.44;
/// `slow` 20.7/+14.25; `Gameplay` 29.9/+20.5).
fn cef_body_metrics(px: f32, unit: f32) -> (f32, f32) {
    let p = px / unit.max(0.01);
    let (ppem, asc) = if p < 21.5 {
        (20.7, 14.25)
    } else if p < 27.0 {
        (23.8, 15.4)
    } else {
        (29.9, 20.5)
    };
    (ppem * unit, asc * unit)
}

/// Body text whose `top` is the legacy glyph-top anchor of the call site.
fn opt_text(
    fb: &mut Framebuffer,
    font: &mut Option<galaxycraft::text::UiFont>,
    x: i32,
    top: i32,
    text: &str,
    px: f32,
    color: Rgb,
) {
    let unit = fb.height as f32 / 1440.0;
    let (ppem, asc) = cef_body_metrics(px, unit);
    opt_text_at(
        fb,
        font,
        x as f32 - 0.375 * unit,
        top as f32 + asc,
        text,
        ppem,
        color,
    );
}

/// Right-aligned [`opt_text`].
fn opt_text_right(
    fb: &mut Framebuffer,
    font: &mut Option<galaxycraft::text::UiFont>,
    right: i32,
    top: i32,
    text: &str,
    px: f32,
    color: Rgb,
) {
    let unit = fb.height as f32 / 1440.0;
    let (ppem, asc) = cef_body_metrics(px, unit);
    let w = font
        .as_ref()
        .map(|f| f.measure_cef(text, ppem, ppem * CEF_TRACKING))
        .unwrap_or(0.0);
    // The CEF right edge sits 0.4 px past the column edge (fast/max/300/
    // Filtered on 130933..130942).
    opt_text_at(
        fb,
        font,
        right as f32 - w + 0.4 * unit,
        top as f32 + asc,
        text,
        ppem,
        color,
    );
}

/// Shared help disc: an anti-aliased r 9 disc in the label colour with a
/// dark "?".  `(cx, cy)` is the continuous centre (130933: 1682.05,349.5).
fn opt_help(
    fb: &mut Framebuffer,
    font: &mut Option<galaxycraft::text::UiFont>,
    cx: f32,
    cy: f32,
    r: f32,
    color: Rgb,
    px: f32,
) {
    aa_circle(fb, cx, cy, r, 0.0, color);
    let qpx = (px * 0.62).max(6.0);
    if let Some(f) = font.as_mut() {
        let w = f.measure_cef("?", qpx, 0.0);
        let ink = Rgb::new(10, 5, 5);
        f.draw_cef(fb, cx - w / 2.0, cy + qpx * 0.36, "?", qpx, 0.0, ink, ink);
    }
}

/// Retail checkbox, measured on 130933 (Mouse Sensitivity box at
/// 1455.7..1485.6 x 340.5..370.5): 2.9 px ring with a 5 px radius,
/// #6c0000 (enabled) or #404040 (disabled), and a 14.9 px mark with a
/// 3.7 px glow ramp, #fcfc4c (enabled) or #808080 (disabled).  `(x, y)`
/// is the nominal 29 px slot; the label starts 13 px after it and sits
/// 8 px below its top.
#[allow(clippy::too_many_arguments)]
fn opt_checkbox(
    fb: &mut Framebuffer,
    font: &mut Option<galaxycraft::text::UiFont>,
    x: i32,
    y: i32,
    s: i32,
    checked: bool,
    enabled: bool,
    label: &str,
    help: bool,
    px: f32,
) {
    let unit = s as f32 / 29.0;
    let (fx, fy) = (x as f32 - 0.3 * unit, y as f32 - 0.5 * unit);
    let b = [fx, fy, fx + 29.9 * unit, fy + 30.0 * unit];
    let (ring, mark) = if enabled {
        (CSS_INNER, CSS_YELLOW)
    } else {
        (Rgb::new(64, 64, 64), Rgb::new(128, 128, 128))
    };
    aa_box(fb, b, 5.0 * unit, 2.9 * unit, ring);
    if checked {
        let (mx, my) = ((b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0);
        let h = 7.45 * unit;
        soft_box(fb, [mx - h, my - h, mx + h, my + h], 0.0, 3.7 * unit, mark);
    }
    let lx = x + s + (13.0 * unit).round() as i32;
    // Checkboxes that title a slider use the white heading colour
    // (130933 Mouse Sensitivity, 130938 Music/Digital Volume).
    let color = if matches!(
        label,
        "Mouse Sensitivity" | "Music Volume" | "Digital Volume"
    ) {
        Rgb::new(255, 255, 255)
    } else if enabled {
        CSS_LAVENDER
    } else {
        Rgb::new(128, 128, 128)
    };
    // Fitted on 130938 "Unit Speech": 23.8 px/em, pen lx-0.125,
    // baseline y+23.375.  The help disc centre sits 13.44 px past the
    // tracked pen end (130933 Mouse Sensitivity disc at 1682.0).
    let _ = px;
    let ppem = 23.8 * unit;
    let pen = lx as f32 - 0.125 * unit;
    opt_text_at(fb, font, pen, y as f32 + 23.375 * unit, label, ppem, color);
    if help
        && let Some(w) = font
            .as_ref()
            .map(|f| f.measure_cef(label, ppem, ppem * CEF_TRACKING))
    {
        opt_help(
            fb,
            font,
            pen + w + 13.44 * unit,
            y as f32 + 8.5 * unit,
            9.0 * unit,
            color,
            ppem,
        );
    }
}

/// Palette of a seven-stop retail slider.
struct SliderLook {
    rail: Rgb,
    stop_fill: Rgb,
    mark: Rgb,
    label: Rgb,
}

/// Seven-stop retail slider, measured on 130933/130942: 6 px rail centred
/// on `cy + 0.5`, stops of radius 12 (3 px ring) from `x0 + 18.4` to
/// `x1 - 17.45`, knob of radius 18 (3 px ring) with an r 8.5 glowing mark.
#[allow(clippy::too_many_arguments)]
fn opt_slider_look(
    fb: &mut Framebuffer,
    font: &mut Option<galaxycraft::text::UiFont>,
    x0: i32,
    x1: i32,
    cy: i32,
    frac: f32,
    left: &str,
    right: &str,
    label_y: i32,
    unit: f32,
    label_px: f32,
    look: &SliderLook,
    stops: usize,
) {
    let yc = cy as f32 + 0.5;
    aa_box(
        fb,
        [
            x0 as f32 - 0.4,
            yc - 3.0 * unit,
            x1 as f32 + 0.5,
            yc + 3.0 * unit,
        ],
        0.0,
        0.0,
        look.rail,
    );
    let sx0 = x0 as f32 + 18.4 * unit;
    let sx1 = x1 as f32 - 17.45 * unit;
    let last = (stops.max(2) - 1) as f32;
    let stop = |i: f32| sx0 + (sx1 - sx0) * i / last;
    for i in 0..stops.max(2) {
        let cx = stop(i as f32);
        aa_circle(fb, cx, yc, 12.0 * unit, 0.0, look.stop_fill);
        aa_circle(fb, cx, yc, 12.0 * unit, 3.0 * unit, look.rail);
    }
    let kx = stop((frac.clamp(0.0, 1.0) * last).round());
    aa_circle(fb, kx, yc, 18.0 * unit, 0.0, look.stop_fill);
    aa_circle(fb, kx, yc, 18.0 * unit, 3.0 * unit, look.rail);
    let r = 8.5 * unit;
    soft_box(
        fb,
        [kx - r, yc - r, kx + r, yc + r],
        r,
        4.0 * unit,
        look.mark,
    );
    opt_text(fb, font, x0, label_y, left, label_px, look.label);
    opt_text_right(fb, font, x1, label_y, right, label_px, look.label);
}

/// Enabled slider: #6c0000 rail, navy stops, yellow mark.
#[allow(clippy::too_many_arguments)]
fn opt_slider(
    fb: &mut Framebuffer,
    font: &mut Option<galaxycraft::text::UiFont>,
    x0: i32,
    x1: i32,
    cy: i32,
    frac: f32,
    left: &str,
    right: &str,
    label_y: i32,
    unit: f32,
    label_px: f32,
) {
    let look = SliderLook {
        rail: CSS_INNER,
        stop_fill: Rgb::new(20, 24, 52),
        mark: CSS_YELLOW,
        label: CSS_LAVENDER,
    };
    opt_slider_look(
        fb, font, x0, x1, cy, frac, left, right, label_y, unit, label_px, &look, 7,
    );
}

/// Enabled slider with `stops` stops (the SD graphics filter has four,
/// screenshot 2026-11-06 15:46:47).
#[allow(clippy::too_many_arguments)]
fn opt_slider_stops(
    fb: &mut Framebuffer,
    font: &mut Option<galaxycraft::text::UiFont>,
    x0: i32,
    x1: i32,
    cy: i32,
    frac: f32,
    left: &str,
    right: &str,
    label_y: i32,
    unit: f32,
    label_px: f32,
    stops: usize,
) {
    let look = SliderLook {
        rail: CSS_INNER,
        stop_fill: Rgb::new(20, 24, 52),
        mark: CSS_YELLOW,
        label: CSS_LAVENDER,
    };
    opt_slider_look(
        fb, font, x0, x1, cy, frac, left, right, label_y, unit, label_px, &look, stops,
    );
}

/// Disabled slider (Gamma Correction): #404040 rail and rings, #0d0d0d
/// stops, grey mark and labels.
#[allow(clippy::too_many_arguments)]
fn opt_slider_grey(
    fb: &mut Framebuffer,
    font: &mut Option<galaxycraft::text::UiFont>,
    x0: i32,
    x1: i32,
    cy: i32,
    frac: f32,
    left: &str,
    right: &str,
    label_y: i32,
    unit: f32,
    label_px: f32,
) {
    let look = SliderLook {
        rail: Rgb::new(64, 64, 64),
        stop_fill: Rgb::new(13, 13, 13),
        mark: Rgb::new(128, 128, 128),
        label: Rgb::new(128, 128, 128),
    };
    opt_slider_look(
        fb, font, x0, x1, cy, frac, left, right, label_y, unit, label_px, &look, 7,
    );
}

/// Retail dropdown, measured on 130942/130949: a 45 px box from `x - 0.4`
/// to `x + w + 0.5` whose top edge `top` is continuous (Video 373.5, Game
/// Console 638.17, Language 449.17); 3 px #6c0000 ring with a 7 px radius,
/// green value at baseline `top + 30` and pen `x + 8.75`, and a 31.8 x 15.6
/// chevron shaded #6f6809 -> #c4c60e, 25.9 px left of the right edge.  A
/// disabled field (Monitor, 130942: 899.64..1416.6 x 475.5..520.5) is a
/// grey #404040 rounded box with no chevron.
#[allow(clippy::too_many_arguments)]
fn opt_dropdown(
    fb: &mut Framebuffer,
    font: &mut Option<galaxycraft::text::UiFont>,
    x: i32,
    top: f32,
    w: i32,
    value: &str,
    enabled: bool,
    unit: f32,
) {
    let h = 45.0 * unit;
    let ink = if enabled {
        let b = [
            x as f32 - 0.4 * unit,
            top,
            (x + w) as f32 + 0.5 * unit,
            top + h,
        ];
        aa_box(fb, b, 7.0 * unit, 3.0 * unit, CSS_INNER);
        aa_chevron(
            fb,
            b[2] - 25.9 * unit,
            b[1] + 16.5 * unit,
            15.9 * unit,
            15.6 * unit,
            Rgb::new(111, 104, 9),
            Rgb::new(196, 198, 14),
        );
        CSS_GREEN
    } else {
        let b = [
            x as f32 - 0.36 * unit,
            top,
            (x + w) as f32 + 0.6 * unit,
            top + h,
        ];
        aa_box(fb, b, 8.0 * unit, 3.0 * unit, Rgb::new(64, 64, 64));
        Rgb::new(128, 128, 128)
    };
    opt_text_at(
        fb,
        font,
        x as f32 + 8.75 * unit,
        top + 30.0 * unit,
        value,
        23.8 * unit,
        ink,
    );
}

/// Retail radio, measured on 130949 (Animated at 899.7..935.4): r 17.85
/// ring of 2.7 px and an r 8.8 glowing yellow mark (4.3 px ramp).
#[allow(clippy::too_many_arguments)]
fn opt_radio(
    fb: &mut Framebuffer,
    font: &mut Option<galaxycraft::text::UiFont>,
    x: i32,
    y: i32,
    s: i32,
    selected: bool,
    enabled: bool,
    label: &str,
    px: f32,
    unit: f32,
) {
    let cx = x as f32 + 17.55 * unit;
    let cy = y as f32 + 18.15 * unit;
    let ring = if enabled {
        CSS_INNER
    } else {
        Rgb::new(64, 64, 64)
    };
    aa_circle(fb, cx, cy, 17.85 * unit, 2.7 * unit, ring);
    if selected {
        let r = 8.8 * unit;
        soft_box(
            fb,
            [cx - r, cy - r, cx + r, cy + r],
            r,
            4.3 * unit,
            CSS_YELLOW,
        );
    }
    let color = if selected { CSS_GREEN_HI } else { CSS_LAVENDER };
    // Fitted on 130949 "Animated": 23.75 px/em, baseline y+25.75.
    let _ = px;
    opt_text_at(
        fb,
        font,
        (x + s) as f32 + 14.5 * unit,
        y as f32 + 25.75 * unit,
        label,
        23.75 * unit,
        color,
    );
}

/// CEF text input (APM threshold, 130949: 1779.4..1855.5 x 642..687.8):
/// 3 px #404040 ring with an 8 px radius and a grey value.
#[allow(clippy::too_many_arguments)]
fn opt_input(
    fb: &mut Framebuffer,
    font: &mut Option<galaxycraft::text::UiFont>,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    value: &str,
    px: f32,
    unit: f32,
) {
    aa_box(
        fb,
        [
            x as f32 - 0.58 * unit,
            y as f32,
            (x + w) as f32 + 0.48 * unit,
            (y + h) as f32 - 0.17 * unit,
        ],
        8.0 * unit,
        3.0 * unit,
        Rgb::new(64, 64, 64),
    );
    // Fitted on 130949 "60": 21.35 px/em, pen x+10, baseline y+29.5.
    let _ = px;
    opt_text_at(
        fb,
        font,
        x as f32 + 10.0 * unit,
        y as f32 + 29.5 * unit,
        value,
        21.35 * unit,
        Rgb::new(170, 170, 176),
    );
}

fn fill_alpha_rect(fb: &mut Framebuffer, x: i32, y: i32, w: i32, h: i32, alpha: u8) {
    for yy in y..y + h {
        for xx in x..x + w {
            let c = fb.get(xx, yy);
            fb.put(xx, yy, c.blend(Rgb::new(0, 0, 0), alpha));
        }
    }
}

/// Signed distance to a rounded rectangle (negative inside).
fn rounded_sdf(px: f32, py: f32, x: f32, y: f32, w: f32, h: f32, r: f32) -> f32 {
    let rx = w / 2.0;
    let ry = h / 2.0;
    let r = r.min(rx).min(ry).max(0.0);
    let qx = ((px - (x + rx)).abs() - (rx - r)).max(0.0);
    let qy = ((py - (y + ry)).abs() - (ry - r)).max(0.0);
    (qx * qx + qy * qy).sqrt() + qx.max(qy).min(0.0) - r
}

/// Exact signed distance to the rounded box `[x0,y0,x1,y1]` (negative
/// inside, true interior distance — unlike [`rounded_sdf`], which clamps
/// the interior to `-r` and so cannot describe bands thicker than `r`).
fn sdf_round_box(px: f32, py: f32, b: [f32; 4], r: f32) -> f32 {
    let hx = (b[2] - b[0]) / 2.0;
    let hy = (b[3] - b[1]) / 2.0;
    let r = r.min(hx).min(hy).max(0.0);
    let qx = (px - (b[0] + hx)).abs() - (hx - r);
    let qy = (py - (b[1] + hy)).abs() - (hy - r);
    let (ox, oy) = (qx.max(0.0), qy.max(0.0));
    (ox * ox + oy * oy).sqrt() + qx.max(qy).min(0.0) - r
}

/// Maps the 2560x1440 content units of the CEF captures to window pixels.
#[derive(Clone, Copy)]
struct CefSpace {
    cx: f32,
    kx: f32,
    ky: f32,
}

impl CefSpace {
    fn rect(&self, b: [f32; 4]) -> [f32; 4] {
        [
            self.cx + b[0] * self.kx,
            b[1] * self.ky,
            self.cx + b[2] * self.kx,
            b[3] * self.ky,
        ]
    }
}

/// CEF border/fill primitive at sub-pixel precision.  The retail Chromium
/// compositor places the Options chrome at fractional positions (bands of
/// 4.6 px #331f1f and 3 px #6c0000 with half-covered edge rows), so the box
/// `b` (content units) is rasterised with 4x4 supersampling.  `t <= 0`
/// fills the box; otherwise only the band `0 < -sdf <= t` is painted.
/// `clip` (content units, union of rects; empty = none) keeps the open
/// sides of the L-shaped plate and of the selected tab out.
#[allow(clippy::too_many_arguments)]
fn cef_band(
    fb: &mut Framebuffer,
    s: CefSpace,
    b: [f32; 4],
    r: f32,
    t: f32,
    color: Rgb,
    opacity: f32,
    clip: &[[f32; 4]],
) {
    let sb = s.rect(b);
    let (rs, ts) = (r * s.ky, t * s.ky);
    let clips: Vec<[f32; 4]> = clip.iter().map(|c| s.rect(*c)).collect();
    let inside_clip = |x: f32, y: f32| {
        clips.is_empty()
            || clips
                .iter()
                .any(|c| x >= c[0] && x < c[2] && y >= c[1] && y < c[3])
    };
    let x0 = sb[0].floor().max(0.0) as i32;
    let y0 = sb[1].floor().max(0.0) as i32;
    let x1 = (sb[2].ceil() as i32).min(fb.width);
    let y1 = (sb[3].ceil() as i32).min(fb.height);
    // A pixel whose whole square is inside a clip rect (or with no clips).
    let square_in_clip = |x: f32, y: f32| {
        clips.is_empty()
            || clips
                .iter()
                .any(|c| x >= c[0] && x + 1.0 <= c[2] && y >= c[1] && y + 1.0 <= c[3])
    };
    for y in y0..y1 {
        for x in x0..x1 {
            let d = sdf_round_box(x as f32 + 0.5, y as f32 + 0.5, sb, rs);
            if d > 1.0 || (ts > 0.0 && d < -(ts + 1.0)) {
                continue;
            }
            // Exact shortcut: the distance is 1-Lipschitz, so with the
            // centre 0.75 px inside the shape (and inside the band's inner
            // edge) every sample of the pixel is inside: full coverage.
            let solid =
                d <= -0.75 && (ts <= 0.0 || d >= -ts + 0.75) && square_in_clip(x as f32, y as f32);
            let mut n = if solid { 16u32 } else { 0 };
            for j in 0..if solid { 0 } else { 4 } {
                for i in 0..4 {
                    let (px, py) = (
                        x as f32 + (i as f32 + 0.5) / 4.0,
                        y as f32 + (j as f32 + 0.5) / 4.0,
                    );
                    let d = sdf_round_box(px, py, sb, rs);
                    if d <= 0.0 && (ts <= 0.0 || d > -ts) && inside_clip(px, py) {
                        n += 1;
                    }
                }
            }
            if n > 0 {
                let a = (n as f32 / 16.0 * opacity * 255.0).round() as u8;
                fb.put(x, y, fb.get(x, y).blend(color, a));
            }
        }
    }
}

/// Window-pixel space for the CEF primitives (controls are positioned in
/// window pixels by their callers).
const PIXEL_SPACE: CefSpace = CefSpace {
    cx: 0.0,
    kx: 1.0,
    ky: 1.0,
};

/// Anti-aliased rounded box (`t <= 0` fills) in window pixels.
fn aa_box(fb: &mut Framebuffer, b: [f32; 4], r: f32, t: f32, color: Rgb) {
    cef_band(fb, PIXEL_SPACE, b, r, t, color, 1.0, &[]);
}

/// Anti-aliased disc (`t <= 0`) or ring of outer radius `r`.
fn aa_circle(fb: &mut Framebuffer, cx: f32, cy: f32, r: f32, t: f32, color: Rgb) {
    aa_box(fb, [cx - r, cy - r, cx + r, cy + r], r, t, color);
}

/// CEF glow fill (the checkbox/radio/slider marks carry a `box-shadow`):
/// the 50 % contour is the rounded box `b`, and coverage ramps linearly
/// across `ramp` pixels around it.
fn soft_box(fb: &mut Framebuffer, b: [f32; 4], r: f32, ramp: f32, color: Rgb) {
    let x0 = (b[0] - ramp).floor().max(0.0) as i32;
    let y0 = (b[1] - ramp).floor().max(0.0) as i32;
    let x1 = ((b[2] + ramp).ceil() as i32).min(fb.width);
    let y1 = ((b[3] + ramp).ceil() as i32).min(fb.height);
    for y in y0..y1 {
        for x in x0..x1 {
            let d = sdf_round_box(x as f32 + 0.5, y as f32 + 0.5, b, r);
            let cov = (0.5 - d / ramp).clamp(0.0, 1.0);
            if cov > 0.0 {
                fb.put(x, y, fb.get(x, y).blend(color, (cov * 255.0).round() as u8));
            }
        }
    }
}

/// Retail dropdown chevron: a downward triangle (top edge `top`, apex at
/// `top + h`, half-width `hw`) shaded top-to-bottom from `c0` to `c1`,
/// 4x4 supersampled.
#[allow(clippy::too_many_arguments)]
fn aa_chevron(fb: &mut Framebuffer, cx: f32, top: f32, hw: f32, h: f32, c0: Rgb, c1: Rgb) {
    let lerp = |a: u32, b: u32, t: f32| (a as f32 + (b as f32 - a as f32) * t).round() as u32;
    for y in top.floor() as i32..(top + h).ceil() as i32 {
        let t = ((y as f32 + 0.5 - top) / h).clamp(0.0, 1.0);
        let (a, b) = (c0.0, c1.0);
        let col = Rgb::new(
            lerp((a >> 16) & 0xff, (b >> 16) & 0xff, t) as u8,
            lerp((a >> 8) & 0xff, (b >> 8) & 0xff, t) as u8,
            lerp(a & 0xff, b & 0xff, t) as u8,
        );
        for x in (cx - hw).floor() as i32..(cx + hw).ceil() as i32 {
            let mut n = 0;
            for j in 0..4 {
                for i in 0..4 {
                    let px = x as f32 + (i as f32 + 0.5) / 4.0;
                    let py = y as f32 + (j as f32 + 0.5) / 4.0;
                    let k = (py - top) / h;
                    if (0.0..=1.0).contains(&k) && (px - cx).abs() <= hw * (1.0 - k) {
                        n += 1;
                    }
                }
            }
            if n > 0 {
                fb.put(
                    x,
                    y,
                    fb.get(x, y)
                        .blend(col, (n as f32 / 16.0 * 255.0).round() as u8),
                );
            }
        }
    }
}

/// Glue `RC_BIGTEXT` (Eurostile regular TTF, kern pairs on) at 1440p:
/// 47.4 px/em, from the mission-row ink widths of 131132 (47.37..47.39).
const GLUE_BIG_PX: f32 = 47.4;

/// In-game HUD text (BlizzardGlobal, fitted on 135051): normal 32.8 px/em
/// (unit name), `RC_SMALLTEXT` 25.4 px/em (stat rows), colour (223,223,255).
/// Face of the in-game HUD text (see `GlueFace::Hud`).
const HUD_FONT: &str = "font/blizzard-regular.ttf";
/// Panel interior in 640x480 UI px (inside the red border of
/// `titleframe_bg`, control 22 at 413,66 205x205).
const MENU_PANEL_RECT: (f32, f32, f32, f32) = (421.0, 76.0, 189.0, 186.0);
/// Offline news panel loading ring, measured on reference picture-7:
/// 57 px across at 3440x1440 content width, centred in the panel, with a
/// dark-green left side and a bright-green right side.
const MENU_SPINNER_RADIUS_UI: f32 = 9.5;
const MENU_SPINNER_STROKE_UI: f32 = 2.0;

/// Typed film caption (1440p content px, Backwater recording 655 s).
/// INFERENCE: face and exact size; colour sampled by eye.
const CAPTION_GREEN: Rgb = Rgb::new(40, 210, 40);
const CAPTION_PX: f32 = 40.0;
const CAPTION_X: f32 = 80.0;
const CAPTION_FIRST_BASE: f32 = 1120.0;
const CAPTION_PITCH: f32 = 50.0;

/// Cinematic subtitle text (1440p content px).  INFERENCE: size and
/// position read by eye from the intro of the Complete UI recording.
const SUBTITLE_PX: f32 = 31.2;
const SUBTITLE_LAST_BASE: f32 = 1395.0;
const SUBTITLE_PITCH: f32 = 40.0;

/// Establishing pages (Boot Camp recording 2026-11-07, 1440p content px):
/// the pale green of both the Rebel Yell copy (10 s) and the mission
/// splash (38 s), the copy's dark band, its BlizzardGlobal size, left edge,
/// first baseline and pitches, and the splash's Eurostile layout.
const BRIEF_GREEN: Rgb = Rgb::new(190, 236, 112);
const BRIEF_BAND_TOP: f32 = 1170.0;
const BRIEF_TEXT_PX: f32 = 31.2;
const BRIEF_TEXT_X: f32 = 460.0;
const BRIEF_TEXT_BASE: f32 = 1228.0;
const BRIEF_LINE_PITCH: f32 = 50.0;
const BRIEF_BLANK_PITCH: f32 = 64.0;
const SPLASH_X: f32 = 65.0;
const SPLASH_PX: f32 = 47.5;
const SPLASH_PITCH: f32 = 58.0;
const SPLASH_LAST_BASE: f32 = 1358.0;

/// How long a command button shows pressed after a click or hotkey (s).
const CARD_PRESS_SECS: f32 = 0.15;

/// How long an order's target flash / ground marker lasts (s).
/// INFERENCE: three blinks, timing not measured on a recording yet.
const ORDER_FLASH_SECS: f32 = 0.75;

/// Whether the order flash is lit `age` seconds after the order: three
/// 0.125 s blinks with 0.125 s gaps.
fn order_flash_on(age: f32) -> bool {
    age < ORDER_FLASH_SECS && ((age / 0.125) as u32).is_multiple_of(2)
}

/// Selection / flash circle colour: green for the player's units, yellow
/// for mineral fields, geysers and neutral units, red for enemies, yellow
/// for allies.  Resources used to come out red (player 11 is not an ally).
fn circle_colour_for(own: bool, neutral: bool, enemy: bool) -> Rgb {
    if own {
        HUD_SELECT_GREEN
    } else if neutral {
        HUD_SELECT_YELLOW
    } else if enemy {
        HUD_SELECT_RED
    } else {
        HUD_SELECT_YELLOW
    }
}

/// Name of a key for [`galaxycraft::menu::game_keystroke`].
fn key_name(key: Key) -> &'static str {
    match key {
        Key::F2 => "F2",
        Key::F3 => "F3",
        Key::F4 => "F4",
        Key::F10 => "F10",
        Key::F11 => "F11",
        Key::C => "C",
        Key::H => "H",
        Key::L => "L",
        Key::M => "M",
        Key::O => "O",
        Key::Q => "Q",
        Key::S => "S",
        Key::X => "X",
        _ => "",
    }
}

/// File stem for a save named `name` in the Save Game dialog: the typed
/// text with path separators and control characters replaced, at most 32
/// characters (the listed name is the stem).
fn save_file_stem(name: &str) -> String {
    let stem: String = name
        .trim()
        .chars()
        .map(|c| {
            if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                c
            }
        })
        .take(32)
        .collect();
    if stem.is_empty() || stem == "." || stem == ".." {
        "save".to_string()
    } else {
        stem
    }
}

/// Two-letter input-language tag retail shows in text fields ("EN" for
/// enUS, Registry New ID at 427 s of the 2026-11-07 recording).
fn input_language_tag(code: &str) -> String {
    code.chars()
        .take(2)
        .collect::<String>()
        .to_ascii_uppercase()
}
/// Footer em size and anchors in 640x480 UI pixels; tuned so the drawn ink
/// box lands on the measured retail one (see `draw_menu_frame_text`).
const BUILD_FOOTER_PX: f32 = 31.0 / 3.0;
const BUILD_FOOTER_RIGHT: f32 = 634.33;
const BUILD_FOOTER_TOP: f32 = 469.0;
const HUD_NAME_PX: f32 = 31.2;
const HUD_SMALL_PX: f32 = 25.6;
/// Baselines below the control's top (content px), from the same fit:
/// name 1192.4 - 1164, "Psi Used" 1279.4 - 1254.
const HUD_NAME_BASE: f32 = 30.9;
const HUD_SMALL_BASE: f32 = 25.4;
const HUD_TEXT_COLOR: Rgb = Rgb::new(223, 223, 255);
const HUD_HP_COLOR: Rgb = Rgb::new(0, 220, 0);
const HUD_MENU_COLOR: Rgb = Rgb::new(255, 255, 71);
/// Resource numbers (135051): normal-size BlizzardGlobal, ink from x +51
/// after the icon's left edge, baseline 40 px below the bar top, in the
/// minimap's own-unit green.
const HUD_RES_TEXT_DX: f32 = 49.0;
const HUD_RES_BASE: f32 = 40.0;
const HUD_RES_COLOR: Rgb = Rgb::new(16, 252, 24);
/// Command glyph tints (enabled yellow, disabled grey).
const HUD_GLYPH_ENABLED: Rgb = Rgb::new(255, 255, 57);
const HUD_GLYPH_DISABLED: Rgb = Rgb::new(150, 150, 150);

/// Opacity of HD shadow images (first pass; fitted below).
const HD_SHADOW_ALPHA: f32 = 0.5;

/// In-game dialog pane, fitted on 134931 against 134905 (same scene).
const DIALOG_FILL: Rgb = Rgb::new(4, 17, 54);
const DIALOG_ALPHA: u8 = 190;
/// Pane border: one game px wide, (16, 18, 73) over the 134905 terrain
/// (~(66, 49, 58)) and tinted over minerals, so translucent at ~93 %.
const DIALOG_EDGE: Rgb = Rgb::new(12, 15, 75);
const DIALOG_EDGE_ALPHA: u8 = 237;
/// Retail pane extent against the dialog rect (content px at 1440p).
/// FACT, identical on 134919 / 134931: x 702..=1878, y 96..=863 for the
/// 384 x 256 `objctdlg` / `tips_dlg` rect drawn at 704..=1855, i.e. 2 px
/// further left and 23 px further right; the height matches.
const GLUE_PANE_DX: f32 = -2.0;
const GLUE_PANE_DW: f32 = 25.0;

/// One pixel of a glue dialog pane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PaneCell {
    Outside,
    Border,
    Fill,
}

/// Classify pane pixel `(dx, dy)` of a `w x h` pane, `g` px per game px.
/// The corners are rounded in whole game px (134919 / 134931, all four
/// corners): the outer row is inset 3 game px, the next two rows 1; the
/// border is the outermost game px of that shape.
fn glue_pane_cell(dx: i32, dy: i32, w: i32, h: i32, g: i32) -> PaneCell {
    let inside = |x: i32, y: i32| {
        if x < 0 || y < 0 || x >= w || y >= h {
            return false;
        }
        let edge = y.min(h - 1 - y);
        let inset = if edge < g {
            3 * g
        } else if edge < 3 * g {
            g
        } else {
            0
        };
        x >= inset && x < w - inset
    };
    if !inside(dx, dy) {
        PaneCell::Outside
    } else if (1..=g).all(|d| {
        inside(dx - d, dy) && inside(dx + d, dy) && inside(dx, dy - d) && inside(dx, dy + d)
    }) {
        PaneCell::Fill
    } else {
        PaneCell::Border
    }
}
const DIALOG_BUTTON_OUTER: Rgb = Rgb::new(12, 72, 204);
const DIALOG_BUTTON_INNER: Rgb = Rgb::new(33, 23, 184);
/// Baselines/offsets in content px (first pass from the 134931 zoom).
const DIALOG_TITLE_BASE: f32 = 48.0;
const DIALOG_TITLE_DX: f32 = -3.0;
const DIALOG_BODY_DX: f32 = -1.0;
const DIALOG_BODY_BASE: f32 = 48.0;
/// One score-screen row.
struct ScoreRow {
    slot: usize,
    human: bool,
    name: String,
    force: String,
    apm: Option<i64>,
    player: galaxycraft::sim::Player,
}

impl ScoreRow {
    /// The three bar columns and the Total Score of a tab.
    fn columns(&self, tab: usize) -> [i64; 4] {
        let p = &self.player;
        let units = (p.scores[1] + p.scores[4]) as i64;
        let structures = (p.scores[2] + p.scores[5]) as i64;
        let resources = (p.minerals_gathered + p.gas_gathered) as i64;
        match tab {
            1 => [
                p.units_produced as i64,
                p.kills.saturating_sub(p.razings) as i64,
                p.units_lost as i64,
                units,
            ],
            2 => [
                p.structures_built as i64,
                p.razings as i64,
                p.buildings_lost as i64,
                structures,
            ],
            3 => [
                p.gas_gathered as i64,
                p.minerals_gathered as i64,
                (p.minerals_spent + p.gas_spent) as i64,
                resources,
            ],
            _ => [units, structures, resources, units + structures + resources],
        }
    }
}

/// Trigger leaderboard: text green and row pitch (one row on the
/// recording; the pitch is INFERENCE), and the BW player colours (the
/// recording's square is (10,70,203) for player 2's (12,72,204)).
const LEADERBOARD_GREEN: Rgb = Rgb::new(72, 249, 74);
const LEADERBOARD_PITCH: f32 = 36.0;
const BW_PLAYER_COLOURS: [Rgb; 8] = [
    Rgb::new(244, 4, 4),
    Rgb::new(12, 72, 204),
    Rgb::new(44, 180, 148),
    Rgb::new(136, 64, 156),
    Rgb::new(248, 140, 20),
    Rgb::new(112, 48, 20),
    Rgb::new(204, 224, 208),
    Rgb::new(252, 252, 56),
];

/// Ticks per real second at the retail Fastest speed (42 ms per tick).
const TICKS_PER_REAL_SECOND: f32 = 1000.0 / galaxycraft::sim::TICK_MS as f32;

/// Countdown display (content px): ink widths 119 px ("25:43") and 92 px
/// ("3:24") fit Eurostile at 45.5 px/em; centre and baseline from the
/// Desperate Alliance recording.
const COUNTDOWN_PX: f32 = 45.5;
const COUNTDOWN_CENTRE_X: f32 = 1418.25;
const COUNTDOWN_BASE: f32 = 46.0;

/// `m:ss`, as the retail timer ("25:43", "3:24", "0:00").
fn countdown_text(seconds: u32) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// Retail in-game error line (`stat_txt.tbl`): baseline and colour from
/// the Wasteland recording; how long it stays is INFERENCE (the recording
/// keeps clicking, so its end is not visible).
const GAME_ERROR_BASE: f32 = 916.0;
const GAME_ERROR_COLOUR: Rgb = Rgb::new(252, 248, 244);
const GAME_ERROR_LIFE: Duration = Duration::from_secs(4);
const STAT_CANT_BUILD_THERE: u16 = 856;
/// (frame w, frame h, box x, box w, box h) the console plates were composed for.
type PlatesKey = (i32, i32, i32, i32, i32);
/// The nine slots of a command card.
type Card = [Option<(
    galaxycraft::button_sets::Button,
    galaxycraft::buttons::ButtonState,
)>; 9];

/// Period of the client's game-end check.  FACT (32-bit dump,
/// 2026-11-06): a global countdown is reset to 0x2d each time it runs out,
/// so the players' end state is looked at once every 46 frames; state 2
/// opens the defeat dialog, 3 / 5 the victory dialog.  The dialog therefore
/// follows the result by 0-45 frames, consistent with the three recordings
/// (1.0, 1.4-1.85 and 1.6-2.9 s with the trigger cycle).
/// INFERENCE: the countdown starts at 0x2d, so the checks fall on frames
/// 46, 92, ... (`tick % 46 == 0`); falsifier: the countdown's initialiser.
const END_CHECK_PERIOD: u64 = 46;

/// Frames from `tick` to the next game-end check (0 on a check frame).
fn frames_to_end_check(tick: u64) -> u32 {
    ((END_CHECK_PERIOD - tick % END_CHECK_PERIOD) % END_CHECK_PERIOD) as u32
}

/// `cmdbtns` frame of the arrow under the unit in training.
const QUEUE_ARROW_FRAME: usize = 11;

/// `units.dat` flag of hero units (Jim Raynor: 0x40 over the plain unit).
const UNIT_FLAG_HERO: u32 = 0x40;
/// `cmdbtns` frames of the multi-selection boxes.
const GROUP_BOX: usize = 13;
const GROUP_BOX_HERO: usize = 19;

/// iscript animation of a Terran building past its scaffold.
const ALMOST_BUILT_ANIM: usize = 15;

/// iscript animations of the three early construction stages: Init,
/// SpecialState1, SpecialState2 (then AlmostBuilt).
const CONSTRUCTION_STAGE_ANIMS: [usize; 3] = [0, 13, 14];

/// How an unfinished Terran building is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConstructionStage {
    /// The `units.dat` construction image at this frame.
    Scaffold(usize),
    /// The building's own image in its AlmostBuilt frame.
    AlmostBuilt,
}

/// FACT (Boot Camp recording, Supply Depot placed at 174.2 s, done at 199.4 s
/// = 600 frames): the scaffold shows frame 0, switches to frame 1 at 177.1 s,
/// frame 2 at 182.7 s and the depot's AlmostBuilt frame at 188.3 s, i.e. at
/// progress 1/9, 1/3 and 5/9.  With hit points growing from 10 % that is
/// 20 / 40 / 60 % HP (INFERENCE that retail keys the stages on HP; falsifier:
/// a repaired or damaged building switching stage off these progress marks).
fn terran_construction_stage(
    u: &SimUnit,
    ty: &galaxycraft::data::UnitType,
) -> Option<ConstructionStage> {
    // The Refinery has no scaffold (`units.dat` construction image 0): it
    // runs the same stages on its own image (iscript 125: Init frame 1,
    // SpecialState1 frame 2, SpecialState2 frame 3, AlmostBuilt frame 4).
    let own_stages = u.type_id == galaxycraft::ids::unit::REFINERY;
    if u.completed
        || !ty.is_building()
        || !((325..=330).contains(&ty.construction_image) || own_stages)
    {
        return None;
    }
    let total = u.build_total.max(1) as i64;
    let done = (total - u.build_remaining.clamp(0, u.build_total) as i64) * 9;
    Some(if done < total {
        ConstructionStage::Scaffold(0)
    } else if done < 3 * total {
        ConstructionStage::Scaffold(1)
    } else if done < 5 * total {
        ConstructionStage::Scaffold(2)
    } else {
        ConstructionStage::AlmostBuilt
    })
}

/// `stat_txt` strings of the selection panel (1-based ids).
const STAT_KILLS: u16 = 764;
const STAT_UPGRADING: u16 = 766;
const STAT_RESEARCHING: u16 = 769;
const STAT_BUILDING: u16 = 772;
const STAT_UNDER_CONSTRUCTION: u16 = 800;
/// `units.dat` weapon id meaning "none".
const NO_WEAPON: u8 = 130;

/// Ticks between minimap terrain/fog recompositions.
const MINIMAP_REFRESH_TICKS: u64 = 8;
/// Card of a structure under construction (Cancel only).
const CONSTRUCTION_CARD: u16 = 231;
/// Retail Esc-only card shown while a target or a building spot is pending.
const TARGET_CARD: u16 = 229;

/// Town-hall supply rows from `stat_txt.tbl`: Used (821), Provided (815),
/// Total (818), Max (824), each Zerg / Terran / Protoss in turn.
fn supply_labels(strings: &galaxycraft::tables::StringTable, race: Race) -> [String; 4] {
    let k = match race {
        Race::Zerg => 0,
        Race::Protoss => 2,
        _ => 1,
    };
    [821u16, 815, 818, 824].map(|id| strings.get(id + k - 1).unwrap_or("").to_string())
}

/// `stat_txt.tbl` id (1-based) of the error a refused order shows:
/// minerals 850, gas 851, supply 844/845/846 by race, energy 864.
fn stat_error_id(e: galaxycraft::sim::SimError, race: Race) -> Option<u16> {
    use galaxycraft::sim::SimError;
    Some(match e {
        SimError::NotEnoughMinerals => 850,
        SimError::NotEnoughGas => 851,
        SimError::NotEnoughSupply => match race {
            Race::Zerg => 844,
            Race::Protoss => 846,
            _ => 845,
        },
        SimError::NotEnoughEnergy => 864,
        _ => return None,
    })
}

/// In-game trigger text (content px, see `draw_hud`): ink widths 603 / 376
/// / 325 px on the recording fit 31.1 px/em; line tops 729 / 777 / 825.
const TRIGGER_TEXT_X: f32 = 77.0;
const TRIGGER_TEXT_BOTTOM: f32 = 847.0;
const TRIGGER_TEXT_PITCH: f32 = 48.0;
const TRIGGER_TEXT_PX: f32 = 31.1;
const TRIGGER_TEXT_COLOUR: Rgb = Rgb::new(214, 210, 240);
/// LAN chat overlay: six newest lines at the upper left of the playfield,
/// the input line centred above the console.  No retail reference exists;
/// the layout follows the game's own text size and margins.
const CHAT_X: f32 = 40.0;
const CHAT_TOP: f32 = 170.0;
const CHAT_PITCH: f32 = 34.0;
const CHAT_PX: f32 = 24.0;
const CHAT_COLOUR: Rgb = Rgb::new(255, 246, 190);

/// Ready-room transmission scroll (content px) `elapsed` ms into a line of
/// `ms` ms whose wrapped text is `text_h` px tall.  FACT (Wasteland
/// recording, row-profile tracking at 10 fps): the text holds still for
/// 4.0 s (Adjutant 10.10 -> 14.12 s, Duke 37.07 -> 41.18 s), then rises
/// linearly until the line ends: 18.23 px/s for 11 lines in 26 469 ms
/// (410 px), 13.34 px/s for 9 lines in 27 794 ms (317 px); both totals
/// are the text height minus ~176 px.  Short texts (Boot Camp's 4 lines)
/// barely move; their rate is INFERENCE from the same rule.
const READY_SCROLL_HOLD_MS: u64 = 4_000;
const READY_SCROLL_KEEP: f32 = 176.0;

fn ready_text_scroll(elapsed_ms: u64, line_ms: u32, text_h: f32) -> f32 {
    let travel = (text_h - READY_SCROLL_KEEP).max(0.0);
    let span = (line_ms as u64).saturating_sub(READY_SCROLL_HOLD_MS).max(1);
    let t = elapsed_ms.saturating_sub(READY_SCROLL_HOLD_MS) as f32 / span as f32;
    travel * t.clamp(0.0, 1.0)
}

/// Score-screen motion, fitted on the retail Wasteland recording
/// (`Screencast_20261105_074531`, backdrop at 7.0 s): the panel's right
/// edge (2187 content px at rest) is at 1653 / 1959 / 2088 at 7.8 / 7.9 /
/// 8.0 s (slide from the left starting 0.63 s in, the remaining offset
/// x0.43 per 0.1 s); the numbers start at 8.375 s and the remaining part
/// shrinks x0.59 per 0.1 s (Overview 112/437/627/739/804/843/865 of 900,
/// the same fractions on every column and again after a tab change).
/// Score screen exit (Backwater Station recording, 4 fps): the panel is
/// out to the left 0.25 s after Ok, the backdrop alone until 1.5 s, black
/// until 2.0 s, then the next screen (the film's caption at 653.0 s).
const SCORE_LEAVE_SLIDE: f32 = 0.25;
const SCORE_LEAVE_BLACK_AT: f32 = 1.5;
const SCORE_LEAVE_SECS: f32 = 2.0;

/// Panel shift (UI px, negative = left) `t` seconds into the exit.
fn score_leave_shift(t: f32) -> f32 {
    -(t / SCORE_LEAVE_SLIDE).min(1.0) * SCORE_SLIDE_FROM * 1.5
}

const SCORE_SLIDE_DELAY: f32 = 0.63;
const SCORE_SLIDE_RATE: f32 = 8.44;
const SCORE_SLIDE_FROM: f32 = 2240.0;
const SCORE_COUNT_DELAY: f32 = 1.375;
const SCORE_COUNT_RATE: f32 = 5.276;

/// Panel x offset (content px) `t` seconds after the score screen showed.
fn score_panel_shift(t: f32) -> f32 {
    if t < SCORE_SLIDE_DELAY {
        return -10_000.0;
    }
    let off = SCORE_SLIDE_FROM * (-(t - SCORE_SLIDE_DELAY) * SCORE_SLIDE_RATE).exp();
    if off < 1.0 { 0.0 } else { -off }
}

/// A score value `t` seconds into its count-up (0 before it starts).
fn score_count(target: i64, t: f32) -> i64 {
    if t <= 0.0 {
        return 0;
    }
    let left = target as f64 * (-(t as f64) * SCORE_COUNT_RATE as f64).exp();
    if left.abs() < 0.5 {
        target
    } else {
        target - left.round() as i64
    }
}

/// Score screen text (content px baselines from each control top).
const SCORE_TITLE_BASE: f32 = 58.0;
const SCORE_TAB_PX: f32 = 40.0;
const SCORE_TAB_BASE: f32 = 64.0;
const SCORE_SMALL_PX: f32 = 30.7;
const SCORE_HEAD_BASE: f32 = 36.0;
const SCORE_TOTAL_HEAD_BASE: f32 = 64.0;
/// Fitted on the recording (9 s): "Garm Brood" ink 238 px (Eurostile at
/// the glue big size), base +47; force names `blizzard-regular` 30.7
/// ("Unidentified Creatures" 327 px) at +68.7 UI, base +90; APM right edge
/// UI 176, base +47 from the row band.
const SCORE_NAME_PX: f32 = GLUE_BIG_PX;
const SCORE_NAME_BASE: f32 = 47.0;
const SCORE_FORCE_DX: f32 = 68.7;
const SCORE_FORCE_BASE: f32 = 90.0;
const SCORE_VALUE_BASE: f32 = 47.0;
const SCORE_APM_RIGHT: f32 = 176.0;
const SCORE_BOX_VALUE_BASE: f32 = 38.0;
const SCORE_BOX_H: f32 = 18.0;
/// Race icon from the name rect (UI px) and the bar fill inset.
const SCORE_ICON_DX: f32 = 7.5;
const SCORE_ICON_DY: f32 = 21.0;
const SCORE_FILL_INSET: f32 = 2.0;
const SCORE_TOTAL_BASE: f32 = 55.0;
const SCORE_ELAPSED_BASE: f32 = 60.0;
const SCORE_BUTTON_BASE: f32 = 64.0;

/// `wmission` message: baselines from the CTEXT top and the pale lavender
/// of the recording ("Congratulations!" / "You are victorious!").
const END_DIALOG_BASE: f32 = 49.0;
const END_DIALOG_PITCH: f32 = 63.0;
const END_DIALOG_TEXT: Rgb = Rgb::new(220, 222, 240);
/// Label pen x from the control's left edge (content px).
const DIALOG_CHECK_DX: f32 = 59.0;
const DIALOG_CHECK_PX: f32 = HUD_NAME_PX;
const DIALOG_CHECK_BORDER: Rgb = Rgb::new(143, 27, 0);
const DIALOG_CHECK_FILL: Rgb = Rgb::new(14, 2, 0);
const DIALOG_CHECK_BASE: f32 = 30.0;
const DIALOG_BODY_PITCH: f32 = 64.0;
/// Button label baseline below the control's centre (content px).  FACT
/// (ink boxes, 2026-11-03): a `DEFPUSHBUTTON` label (`OK`, 134931) sits
/// 6 px lower than `PUSHBUTTON` ones (`Previous Tip` / `Next Tip`, and
/// 134919's `Previous (Esc)`, which has the bright focus rim but is a plain
/// `PUSHBUTTON`); one shared 16 put them 2 and 7-8 px low.
const DIALOG_BUTTON_BASE: f32 = 8.0;
const DIALOG_DEFAULT_BUTTON_BASE: f32 = 14.0;
const DIALOG_SHADOW: f32 = 2.0;

/// Label baseline offset of a dialog button of layout type `kind`.
/// Label baselines of the in-game menu and its prompts (`gamemenu`,
/// `abrtmenu`, `quit`, `quit2mnu`, `restart`), measured on the 2026-11-07
/// recording at 491 s (ink boxes against our render): "Save Game" sat 6 px
/// high and "Return to Game (Esc)" 5 px low with the tips values.
const GAME_MENU_BUTTON_BASE: f32 = 14.0;
const GAME_MENU_DEFAULT_BUTTON_BASE: f32 = 9.0;

/// [`button_label_base`] for a dialog: the in-game menus use their own.
fn dialog_button_base(dialog: &str, kind: &str) -> f32 {
    if matches!(
        dialog,
        "gamemenu" | "abrtmenu" | "quit" | "quit2mnu" | "restart" | "helpmenu"
    ) {
        if kind == "DEFPUSHBUTTON" {
            GAME_MENU_DEFAULT_BUTTON_BASE
        } else {
            GAME_MENU_BUTTON_BASE
        }
    } else {
        button_label_base(kind)
    }
}

fn button_label_base(kind: &str) -> f32 {
    if kind == "DEFPUSHBUTTON" {
        DIALOG_DEFAULT_BUTTON_BASE
    } else {
        DIALOG_BUTTON_BASE
    }
}

/// Playfield height above the console framed by the minimap box (game px).
const MINIMAP_VIEW_H: i32 = 368;

/// `upgrades.dat` id of Protoss Plasma Shields.
const PLASMA_SHIELDS: u8 = 15;

/// Upgrade level digit: pen x before the button's right edge and baseline
/// above its bottom (content px; measured on 135051: ink x 1120, y 1396).
const UPGRADE_LEVEL_DX: f32 = 38.0;
const UPGRADE_LEVEL_DY: f32 = 11.0;

/// Drawn offsets of the upgrade button and the radar buttons from their
/// `statdata` / `minimap` rects (content px; shift search on 135051).  The
/// upgrade x was re-measured after `statdata` moved to whole game px
/// (2026-11-03: +2 px on 135051 / 135109, box error 21.2 -> 5.3).
const UPGRADE_DRAW_SHIFT: (f32, f32) = (-4.0, -6.0);
const MINIMAP_BUTTON_SHIFT: (f32, f32) = (8.0, 12.0);

/// Drawn offset of the command-card buttons from their `statbtndefault`
/// rects (content px at 1440p; the hit rects stay).  FACT (2026-11-03,
/// shift search per button): Zerg (134905, three buttons) and Protoss
/// (135051, two) sit 3 px left and 0.25-1 px lower; Terran (135020) is
/// within 0.5 px.  Frame mean with (0, 0) / (-3, 0) / (-3, +1): 2.82 /
/// 2.69 / 2.67 (134905), 3.43 / 3.32 / 3.31 (135051), 3.56 / 3.46 / 3.45
/// (135114).  The race layouts and `cmdbtns` sizes are identical, so the
/// cause is not in the data read so far.
fn card_draw_shift(race: Race) -> (f32, f32) {
    match race {
        Race::Terran => (0.0, 0.0),
        _ => (-3.0, 1.0),
    }
}

/// MENU button plate: `dlg.dds.grp` frame 0 of the console race (plain
/// metal / bracketed blue / rock; SD frame 0 is the 64x20 control itself)
/// at 0.75, drawn from the `stat_f10` control's top-left shifted by this
/// offset (content px at 1440p).  FACT: template match of the 180x72 plate
/// on 135020 / 135051 / 134905 lands on the same (1885, 1164) for all three
/// races, i.e. (-3, 0) from the control at (1888, 1164); box error 34.3 /
/// 9.7 / 16.1 -> 8.3 / 7.0 / 7.4 before the label goes on top.
const MENU_PLATE_FRAME: usize = 0;
const MENU_PLATE_SHIFT: (f32, f32) = (-3.0, 0.0);

/// Game-px x offset of a console dialog in the widened 16:9 frame.  Retail
/// keeps the classic 640x480 layout x3 and anchors each dialog: the minimap
/// stays left, `statdata` is centred and the right-hand dialogs (`statres`,
/// `statport`, `stat_f10`, `statbtn*`) move by the whole extra width.
/// Measured on 135051: card button 1 at x 2155 = (505 + 213.33) * 3.
///
/// `statdata` is centred in whole game px: the view is `view_w` wide (853
/// at 16:9), so it moves by (853 - 640) / 2 = 106, not 106.67.  FACT
/// (2026-11-03, shift search on 134905 / 135051): with 106.67 the
/// wireframe sat 1.75 px and every centred string ~2 px right of retail;
/// with 106 the wireframe is within 0.25 px (the earlier "name centred at
/// 1265 with 106.67" was compensating a text-measure error).  The
/// right-hand dialogs and the glue dialogs over the playfield (`objctdlg`,
/// `tips_dlg`) keep the fractional screen-px widening: integer there
/// worsens 135020 / 134919 / 134931.
/// Game-tab "Mini-Map Position": `Standard` moves the minimap and its
/// buttons from the screen's left corner towards the centre of a widescreen
/// playfield, matching the retail `MinimapPosition` setting.  At 4:3 the
/// shift is zero.
fn minimap_centering_shift(position: u8, dialog: &str, content_w: i32, s: f32) -> f32 {
    if position != 1 || dialog != "minimap" {
        return 0.0;
    }
    ((content_w as f32 / s - 640.0) / 2.0).max(0.0)
}

fn console_dialog_shift(dialog: &str, view_w: i32, content_w: i32, s: f32) -> f32 {
    let screen_extra = content_w as f32 / s - 640.0;
    match dialog {
        "minimap" => 0.0,
        "statdata" => ((view_w - 640).max(0) / 2) as f32,
        "objctdlg" | "tips_dlg" | "wmission" | "lmission" | "gamemenu" | "abrtmenu" | "quit"
        | "quit2mnu" | "restart" | "helpmenu" | "help" | "savegame" | "loadgame" => {
            screen_extra / 2.0
        }
        _ => screen_extra,
    }
}

/// Screen rect `(x, y, w, h)` of the MENU plate for the `stat_f10`
/// control rect `r` and a `w x h` 4x frame, `k` = content height / 1440.
fn menu_plate_rect(r: UiRect, w: usize, h: usize, k: f32) -> (i32, i32, i32, i32) {
    (
        r.x + (MENU_PLATE_SHIFT.0 * k).round() as i32,
        r.y + (MENU_PLATE_SHIFT.1 * k).round() as i32,
        (w as f32 * 0.75 * k).round() as i32,
        (h as f32 * 0.75 * k).round() as i32,
    )
}

/// Portrait clip rect and frame-centre shift relative to `statport`
/// control 1 (content px at 1440p, measured; see `draw_hud_portrait`).
const PORTRAIT_FACE: (f32, f32, f32, f32) = (11.0, 17.0, 160.0, 164.0);
const PORTRAIT_FRAME_SHIFT: (f32, f32) = (-9.0, 18.0);

/// Own selection circle and HP fill (134905 / 135051).
const HUD_SELECT_GREEN: Rgb = Rgb::new(36, 152, 36);
/// Selection circle of an enemy / an allied or neutral unit.  INFERENCE:
/// the hues mirror the measured green.
const HUD_SELECT_RED: Rgb = Rgb::new(176, 28, 28);
const HUD_SELECT_YELLOW: Rgb = Rgb::new(196, 176, 28);
const HP_BAR_BORDER: Rgb = Rgb::new(8, 8, 8);
const HP_BAR_SHIELD: Rgb = Rgb::new(12, 72, 204);

/// Retail health bar (OpenBW `draw_health_bars`) at screen resolution:
/// `width = size - (size - 1) % 3` (min 19) game px, 5 rows (7 with
/// shields), centred at `centre`.  HD look measured on 134905 / 135051:
/// 1-game-px (8,8,8) borders, flat fill rows, and every 3 game px a 2 px
/// dark separator with half-blended pixels on both sides.
/// INFERENCE: the yellow/red fills below 66 % / 33 % are unmeasured and use
/// the SD thresholds with retail-like hues.
fn draw_health_bar(
    fb: &mut Framebuffer,
    centre: (f32, f32),
    size: i32,
    hp_pct: i32,
    shield_pct: Option<i32>,
    s: f32,
    clip: (i32, i32, i32, i32),
) {
    let mut width = size - (size - 1).rem_euclid(3);
    width = width.max(19);
    let rows = if shield_pct.is_some() { 7 } else { 5 };
    let filled = |pct: i32| {
        let r = pct.clamp(0, 100) * width / 100;
        if r < 3 {
            3
        } else if r % 3 > 1 {
            r + 3 - r % 3
        } else {
            r - r % 3
        }
    };
    let hp_fill = if hp_pct >= 66 {
        HUD_SELECT_GREEN
    } else if hp_pct >= 33 {
        Rgb::new(200, 180, 36)
    } else {
        Rgb::new(200, 36, 36)
    };
    let x0 = (centre.0 - (width / 2) as f32 * s).round() as i32;
    let y0 = (centre.1 - (rows / 2) as f32 * s).round() as i32;
    let w = (width as f32 * s).round() as i32;
    let h = (rows as f32 * s).round() as i32;
    let period = (3.0 * s).round().max(3.0) as i32;
    for py in y0.max(clip.1)..(y0 + h).min(clip.3) {
        let row = ((py - y0) as f32 / s) as i32;
        let (fill, pct) = match (rows, row) {
            (7, 1) | (7, 2) => (Some(HP_BAR_SHIELD), shield_pct.unwrap_or(0)),
            (7, 4) | (7, 5) | (5, 1..=3) => (Some(hp_fill), hp_pct),
            _ => (None, 0),
        };
        let fill_w = (filled(pct) as f32 * s).round() as i32;
        for px in x0.max(clip.0)..(x0 + w).min(clip.2) {
            let o = px - x0;
            let c = match fill {
                None => HP_BAR_BORDER,
                Some(c) => {
                    let c = if o < fill_w { c } else { HP_BAR_BORDER };
                    match o % period {
                        1 | 2 => HP_BAR_BORDER,
                        0 | 3 => c.blend(HP_BAR_BORDER, 128),
                        _ => c,
                    }
                }
            };
            fb.put(px, py, c);
        }
    }
}

/// HUD element with a tooltip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HudHover {
    Card(usize),
    Shields,
    MinimapTerrain,
}

/// `stat_txt` string of the minimap terrain toggle's tooltip: 809 "Show
/// Terrain in Minimap" while hidden, 810 "Hide Terrain ..." while shown.
fn minimap_toggle_string(terrain_shown: bool) -> u16 {
    if terrain_shown { 810 } else { 809 }
}

/// Tooltip style measured on 135109 / 135114 / 135121 (content px).  The
/// text is the HUD face at the stat-row size (2026-11-03: glyph edges of
/// "Protoss Plasma Shields" / "Shields: 0" within 1.3 px at 25.6 px/em,
/// 4.8 px of drift at the earlier 26.1).
const TOOLTIP_PX: f32 = HUD_SMALL_PX;
const TOOLTIP_PAD_L: f32 = 16.0;
const TOOLTIP_PAD_R: f32 = 14.0;
const TOOLTIP_LINE0_H: f32 = 52.0;
const TOOLTIP_SINGLE_H: f32 = 59.0;
const TOOLTIP_PITCH: f32 = 38.0;
const TOOLTIP_BASE: f32 = 34.0;
const TOOLTIP_COST_H: f32 = 56.0;
/// Cost row top below the first line, and the gap after each value
/// (135114: icons 9 px lower and the supply icon 5 px closer than a
/// first pass).
const TOOLTIP_COST_DY: f32 = 3.0;
const TOOLTIP_COST_GAP: f32 = 7.0;
const TOOLTIP_ICON: f32 = 40.0;
const TOOLTIP_RADIUS: f32 = 15.0;
const TOOLTIP_RIM: f32 = 3.5;
const TOOLTIP_FILL: Rgb = Rgb::new(4, 4, 4);
const TOOLTIP_RIM_COLOR: Rgb = Rgb::new(12, 72, 204);
const TOOLTIP_HOTKEY: Rgb = Rgb::new(252, 252, 56);

/// Retail button tooltip from a `stat_txt` button string
/// (`p\x01Build \x03P\x01robe` -> `Build Probe [\x03P\x01]`): the leading
/// hotkey character is dropped, the inline highlight removed and the key
/// appended in brackets.
fn button_tooltip_text(raw: &str) -> String {
    let mut chars = raw.chars();
    let hotkey = chars.next().unwrap_or(' ');
    let body: String = chars.filter(|c| (*c as u32) >= 0x20).collect();
    if hotkey.is_ascii_alphanumeric() {
        format!("{body} [\u{3}{}\u{1}]", hotkey.to_ascii_uppercase())
    } else {
        body
    }
}

/// Text without its control bytes.
fn strip_codes(s: &str) -> String {
    s.chars().filter(|c| (*c as u32) >= 0x20).collect()
}

/// Rounded rectangle with a rim, anti-aliased by 4x4 supersampling of the
/// corner discs (`radius`, `rim` in window px).
fn draw_rounded_box(
    fb: &mut Framebuffer,
    rect: (i32, i32, i32, i32),
    radius: f32,
    rim: f32,
    fill: Rgb,
    rim_color: Rgb,
) {
    draw_rounded_box_alpha(fb, rect, radius, rim, fill, rim_color, 255);
}

/// [`draw_rounded_box`] with a coverage multiplier, so a plate can be
/// translucent without changing the anti-aliased edge.
fn draw_rounded_box_alpha(
    fb: &mut Framebuffer,
    rect: (i32, i32, i32, i32),
    radius: f32,
    rim: f32,
    fill: Rgb,
    rim_color: Rgb,
    alpha: u8,
) {
    let (x0, y0, w, h) = rect;
    let (fw, fh) = (w as f32, h as f32);
    let r = radius.min(fw / 2.0).min(fh / 2.0);
    // Signed distance to the rounded rectangle at a sample point.
    let dist = |px: f32, py: f32| {
        let qx = (px - fw / 2.0).abs() - (fw / 2.0 - r);
        let qy = (py - fh / 2.0).abs() - (fh / 2.0 - r);
        let ox = qx.max(0.0);
        let oy = qy.max(0.0);
        (ox * ox + oy * oy).sqrt() + qx.max(qy).min(0.0) - r
    };
    for y in 0..h {
        for x in 0..w {
            let (mut inside, mut core) = (0u32, 0u32);
            for j in 0..4 {
                for i in 0..4 {
                    let d = dist(
                        x as f32 + (i as f32 + 0.5) / 4.0,
                        y as f32 + (j as f32 + 0.5) / 4.0,
                    );
                    if d <= 0.0 {
                        inside += 1;
                        if d <= -rim {
                            core += 1;
                        }
                    }
                }
            }
            if inside == 0 {
                continue;
            }
            let colour = rim_color.blend(fill, (core * 255 / inside.max(1)) as u8);
            let (qx, qy) = (x0 + x, y0 + y);
            let bg = fb.get(qx, qy);
            let coverage = (inside * 255 / 16) * alpha as u32 / 255;
            fb.put(qx, qy, bg.blend(colour, coverage as u8));
        }
    }
}

/// Rim of a rounded rectangle only (4x4 supersampled), leaving the
/// inside untouched.
fn draw_rounded_rim(
    fb: &mut Framebuffer,
    rect: (i32, i32, i32, i32),
    radius: f32,
    rim: f32,
    colour: Rgb,
) {
    let (x0, y0, w, h) = rect;
    let (fw, fh) = (w as f32, h as f32);
    let r = radius.min(fw / 2.0).min(fh / 2.0);
    let dist = |px: f32, py: f32| {
        let qx = (px - fw / 2.0).abs() - (fw / 2.0 - r);
        let qy = (py - fh / 2.0).abs() - (fh / 2.0 - r);
        let (ox, oy) = (qx.max(0.0), qy.max(0.0));
        (ox * ox + oy * oy).sqrt() + qx.max(qy).min(0.0) - r
    };
    for y in 0..h {
        for x in 0..w {
            let mut n = 0u32;
            for j in 0..4 {
                for i in 0..4 {
                    let d = dist(
                        x as f32 + (i as f32 + 0.5) / 4.0,
                        y as f32 + (j as f32 + 0.5) / 4.0,
                    );
                    if d <= 0.0 && d > -rim {
                        n += 1;
                    }
                }
            }
            if n > 0 {
                let bg = fb.get(x0 + x, y0 + y);
                fb.put(x0 + x, y0 + y, bg.blend(colour, (n * 255 / 16) as u8));
            }
        }
    }
}

/// Draw an HD glyph tinted by `colour` (its luminance scales the tint),
/// keeping the glyph's alpha.
fn draw_tinted(
    fb: &mut Framebuffer,
    img: &galaxycraft::dds::DdsImage,
    dest: (i32, i32, i32, i32),
    colour: Rgb,
) {
    let (dx, dy, dw, dh) = dest;
    if dw <= 0 || dh <= 0 || img.width == 0 || img.height == 0 {
        return;
    }
    for y in 0..dh {
        let sy = ((y as f32 + 0.5) * img.height as f32 / dh as f32) as usize;
        for x in 0..dw {
            let sx = ((x as f32 + 0.5) * img.width as f32 / dw as f32) as usize;
            let p = img.pixel(sx.min(img.width - 1), sy.min(img.height - 1));
            let a = (p >> 24) as u8;
            if a == 0 {
                continue;
            }
            // Retail saturates the glyph shading: luminance >= 0.5 is the
            // full tint (fitted on 135051: 0.35 -> ~0.8, >= 0.5 -> 1.0).
            let lum = (((p >> 16) & 0xff).max((p >> 8) & 0xff).max(p & 0xff) as f32 / 255.0 * 2.0)
                .min(1.0);
            let c = Rgb::new(
                (((colour.0 >> 16) & 0xff) as f32 * lum) as u8,
                (((colour.0 >> 8) & 0xff) as f32 * lum) as u8,
                ((colour.0 & 0xff) as f32 * lum) as u8,
            );
            let bg = fb.get(dx + x, dy + y);
            fb.put(dx + x, dy + y, bg.blend(c, a));
        }
    }
}

/// Creep variants of the 134905 capture, tiles (60, 3) onward: the retail
/// client picks each creep tile's subtile from its session RNG, so they are
/// recorded capture state (best of the 13 HD creep megatiles per tile, mean
/// error 2.6 vs 7.9 hashed; 128 of 132 tiles use 0-5, as the 96/4 rule
/// predicts).  `.` = not observed (covered or off screen).  Rows 13-14
/// (the band above the console, 2026-11-03): best of 13 renders per tile
/// on the pixels where the variants differ, kept only with error < 3.5 and
/// a margin > 2.5 over the runner-up (accepted: 0.8-2.8 error, 3.0-7.8
/// margin); error over x 600-2100, y 900-1080: 5.99 -> 1.17 on all three
/// Zerg captures.  Nine more tiles around the Hatchery (2026-11-03,
/// partly under its sprite): best of 13 renders on the pixels where the
/// variants differ, same acceptance (error < 3.5, margin > 2.5); 11 other
/// visible tiles failed it and stay unrecorded.
const HUNTERS_ZERG_CREEP: ((i32, i32), &[&str]) = (
    (60, 3),
    &[
        "..............................",
        "........22140..444210.........",
        "......0033214504301401........",
        ".....33520553323540...........",
        ".....b5013033230522...........",
        "....342207453.5505355434......",
        "....431035211..930534334......",
        "....035634231.1..3044410......",
        ".....03a5144....1020431.......",
        ".....450405134232331324.......",
        "......5523384431512019........",
        "........445044052320..........",
    ],
);

/// Geyser smoke (`image, frame, LO dx, LO dy`) of a Hunters capture:
/// `creategasoverlays n` draws image 430 + n at the geyser's LO offset n, and
/// which overlay and frame show is the capture's moment.  Matched over all
/// five smoke images x 8 frames: overlay 2 at (+34, +2.5) in the Protoss
/// (135051) and Terran games, overlay 0 at (+11.4, -19.3) in the Zerg one.
/// The later Protoss captures (2026-11-03, same matcher on a smoke-free
/// render): 135109 shows overlay 0 frame 3 at (+11.3, -18.5) -- the Zerg
/// game's overlay-0 offset again -- and 135114 / 135121 show none (no
/// candidate lowers the error; smoke region 6.13 -> 4.37 without it).  The
/// Zerg dialog captures 134919 / 134931 show none either (the matcher finds
/// 134905's overlay 0 frame 6 on 134905 itself, nothing on the other two).
fn hunters_smoke(race: Race) -> Option<(u16, usize, f32, f32)> {
    Some(match race {
        Race::Zerg => (430, 6, 11.4, -19.3),
        Race::Protoss => (432, 6, 34.25, 2.5),
        _ => (432, 4, 33.75, 2.5),
    })
}

/// Whether a Hunters capture shows the terrain minimap.  Capture state:
/// 134905 shows the visible terrain and creep in its minimap, and so does
/// 135121 (the capture hovering the terrain toggle), while 134919 / 134931
/// (later in the Zerg game) and the other Protoss / Terran captures show a
/// black one.  Minimap region error on 134905: 1.07 -> 0.75 with terrain;
/// enabling it on 134919 worsens the frame (2.697 -> 2.720).
fn hunters_capture_minimap_terrain(shot: &str) -> bool {
    matches!(shot, "ingame-zerg" | "ingame-protoss-tip-minimap")
}

/// Per-capture override of [`hunters_smoke`] for the later Protoss shots.
fn hunters_capture_smoke(shot: &str) -> Option<Option<(u16, usize, f32, f32)>> {
    match shot {
        "ingame-protoss-tip-shields" => Some(Some((430, 3, 11.4, -19.3))),
        "ingame-protoss-tip-probe" | "ingame-protoss-tip-minimap" | "objectives" | "tips" => {
            Some(None)
        }
        _ => None,
    }
}

/// Start units' poses in the three Hunters captures (type, x, y, facing).
/// The workers had started moving and the Zerg units idle-turn at random,
/// so these are recorded capture state: masked template match of every HD
/// direction around each unit, lit like the renderer (`/tmp` tooling,
/// 2026-11-03; RMS 25-50 per unit).
fn hunters_poses(race: Race) -> &'static [(u16, i32, i32, u8)] {
    use galaxycraft::ids::unit as u;
    match race {
        Race::Protoss => &[
            (u::PROBE, 328, 3759, 240),
            (u::PROBE, 360, 3752, 120),
            (u::PROBE, 384, 3752, 240),
            (u::PROBE, 418, 3744, 232),
        ],
        Race::Terran => &[
            (u::SCV, 272, 1608, 240),
            (u::SCV, 296, 1608, 104),
            (u::SCV, 320, 1608, 128),
            (u::SCV, 344, 1608, 56),
        ],
        _ => &[
            (u::DRONE, 2329, 352, 104),
            (u::DRONE, 2393, 352, 104),
            (u::DRONE, 2417, 352, 104),
            (u::DRONE, 2433, 322, 104),
            (u::OVERLORD, 2269, 371, 104),
            (u::LARVA, 2340, 339, 184),
            (u::LARVA, 2340, 339, 184),
        ],
    }
}

/// Parse a hex-digit tile grid (`.` = none) starting at `origin`.
fn parse_tile_grid(origin: (i32, i32), rows: &[&str]) -> std::collections::HashMap<(i32, i32), u8> {
    let mut out = std::collections::HashMap::new();
    for (dy, row) in rows.iter().enumerate() {
        for (dx, ch) in row.chars().enumerate() {
            if let Some(v) = ch.to_digit(16) {
                out.insert((origin.0 + dx as i32, origin.1 + dy as i32), v as u8);
            }
        }
    }
    out
}

/// The newest Transmission / Talking Portrait `(unit, ms)` of `player`
/// recorded after `seen` (indices into the two trigger lists), and the new
/// `seen`.  Lists shorter than `seen` (a new world) start over.
fn latest_trigger_portrait(
    transmissions: &[(usize, u16, u32, u32)],
    talking: &[(usize, u16, u32)],
    seen: (usize, usize),
    player: usize,
) -> (Option<(u16, u32)>, (usize, usize)) {
    let from_tx = if seen.0 > transmissions.len() {
        0
    } else {
        seen.0
    };
    let from_tp = if seen.1 > talking.len() { 0 } else { seen.1 };
    let latest = transmissions[from_tx..]
        .iter()
        .map(|&(p, unit, _, ms)| (p, unit, ms))
        .chain(talking[from_tp..].iter().copied())
        .rfind(|&(p, _, _)| p == player)
        .map(|(_, unit, ms)| (unit, ms));
    (latest, (transmissions.len(), talking.len()))
}

/// Hosted LAN lobby rows: a header, then one row per seat (the host first,
/// joined clients in peer order, open seats last).
fn lan_lobby_lines(queue: &galaxycraft::net::JoinQueue, host_race: Race) -> Vec<String> {
    let race = |r: Race| format!("{r:?}");
    let mut out = vec![format!(
        "Lobby {}/{} players - ENTER starts",
        queue.players(),
        queue.capacity()
    )];
    out.push(format!("  1. You (host) - {}", race(host_race)));
    for (i, (addr, r)) in queue.members().iter().enumerate() {
        out.push(format!("  {}. {} - {}", i + 2, addr.ip(), race(*r)));
    }
    for seat in queue.players()..queue.capacity() {
        out.push(format!("  {}. (open)", seat + 1));
    }
    out
}

/// A small window with `lines` of text, open until it is closed or Escape
/// is pressed (used when no game data can be found).
fn show_message_window(lines: &[&str]) {
    let (w, h) = (900usize, 200usize);
    let Ok(mut window) = Window::new("GalaxyCraft", w, h, WindowOptions::default()) else {
        return;
    };
    let mut fb = Framebuffer::new(w, h);
    fb.clear(Rgb::new(0, 0, 0));
    for (i, line) in lines.iter().enumerate() {
        fb.text(24, 40 + i as i32 * 28, line, Rgb::new(220, 220, 220), 2);
    }
    window.set_target_fps(30);
    while window.is_open() && !window.is_key_down(Key::Escape) {
        if window.update_with_buffer(&fb.pixels, w, h).is_err() {
            break;
        }
    }
}

/// Seed of the client's CRT stream: retail calls `srand(time(NULL))`
/// (64-bit 1.23.10, `0x2b17e2`); headless captures use a fixed seed so they
/// stay reproducible.
fn crt_session_seed(headless: bool) -> u32 {
    if headless {
        return 0;
    }
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0)
}

/// Retail melee start camera: start location 200 game px below the top.
const START_CAMERA_TOP: i32 = 200;
/// Glue text faces: `RC_SMALLTEXT` / plain buttons (BlizzardGlobal),
/// `RC_BIGTEXT` (Eurostile) and `RC_BIGGERTEXT` (Eurostile Extended).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GlueFace {
    Small,
    Big,
    Bigger,
    /// In-game HUD text: `font/blizzard-regular.ttf` with linear advances
    /// and its kern pairs (no tracking).  FACT (2026-11-03, glyph left edges
    /// of the eight 134905 / 135051 stat rows and both unit names): mean
    /// error 0.42 px at 25.6 px/em and 0.33 px at 31.2 px/em, against 1.65 px
    /// for the earlier model (`bl.ttf`, no kern table, advances rounded up to
    /// whole pixels, which was compensating for the wrong face).  Kerning
    /// lowers every in-game frame (avg 2.980 -> 2.955).
    Hud,
}

/// Ready-room text (content px): portrait face inside a slot (UI px
/// offset/size), transmission/objective text size, first baseline and
/// pitch, and the "Objectives" heading.  First fit on the retail Wasteland
/// recording frame at 50 s (`Screencast_20261105_073713`).
/// Faces fitted by ink width: transmission `blizzard-regular` 35.1 px/em
/// (first line 630 px), objectives 30.7 (first objective 265 px, 45 px
/// pitch), heading Eurostile 46.7 ("Objectives" = 198 px); text colour the
/// glue lavender.
/// The retail room fades in at 8.05 s of the Wasteland recording and its
/// first portrait shows at 9.10 s (script time 0), the first line at
/// 10.10 s (after the script's 1000 ms Wait).
const READY_SCRIPT_DELAY_MS: u64 = 1050;
/// The room's contents show this long after it appears (Boot Camp
/// recording: plates in place at 38.6 s, contents at 39.1 s of a room
/// shown at 38.3 s).
const READY_BUILD_SECS: f32 = 0.8;
/// Portrait inside a slot, filling the `tframe` red border (UI px from
/// the slot origin): border 440..779 x 305..634 content px for slot 0.
const READY_PORTRAIT_FACE: (f32, f32, f32, f32) = (7.0, 3.5, 113.0, 110.0);
/// Darkening of the scanline rows (alpha of black).
const READY_SCANLINE_DIM: u8 = 120;
const READY_TEXT_PX: f32 = 35.1;
const READY_TEXT_BASE: f32 = 38.5;
const READY_TEXT_PITCH: f32 = 54.0;
const READY_OBJ_PX: f32 = 30.7;
const READY_OBJ_BASE: f32 = 30.0;
const READY_OBJ_PITCH: f32 = 45.0;
const READY_HEAD_PX: f32 = 46.7;
const READY_HEAD_BASE: f32 = 50.0;

/// Glue `RC_SMALLTEXT` / plain `FLCBTN` labels (BlizzardGlobal), fitted on
/// 131132: title 33.4 px/em (ink 747..1086), Ok/Cancel 34.0 px/em.
const GLUE_SMALL_PX: f32 = 33.4;
const GLUE_SMALL_BUTTON_PX: f32 = 34.0;
/// Glue `RC_BIGGERTEXT` (Eurostile Extended): 47.5 px/em, solved from the
/// measured ink widths of the ten campaign labels (47.3..48.0).
const GLUE_BIGGER_PX: f32 = 47.5;
/// Cap height of both Eurostile faces (OS/2 `sCapHeight` 667/1000).
const GLUE_CAP: f32 = 0.667;
/// Glue colours: idle green, bright hotkey/selection green and the
/// white-lavender info text (the `\x02` episode line, captions, prompts).
const GLUE_GREEN: Rgb = Rgb::new(102, 227, 53);
const GLUE_BRIGHT: Rgb = Rgb::new(208, 255, 131);
const GLUE_INFO: Rgb = Rgb::new(198, 218, 255);
/// Extra advance of the glue text, as for the CEF body text.
const GLUE_TRACKING: f32 = 0.00125;
/// Locked-slot caption baselines (130757: ink rows 616..656 / 680..720).
const GLUE_CAPTION_BASE: f32 = 647.5;
const GLUE_CAPTION_PITCH: f32 = 64.0;
/// `glupokcancel` body: pen x, first baseline and line pitch (live 08:
/// ink bottoms 506 / 570 / 699 for lines 1, 2 and 4).
const GLUE_PROMPT_X: f32 = 771.5;
const GLUE_PROMPT_BASE: f32 = 497.6;
const GLUE_PROMPT_PITCH: f32 = 64.3;
/// Offset of a plain `FLCBTN` label from its rect centre (content px).
const GLUE_SMALL_BUTTON_DX: f32 = 2.0;
const GLUE_SMALL_BUTTON_DY: f32 = 1.0;
/// Mission list rows: pen x and first baseline (57 px pitch).
const GLUE_ROW_X: f32 = 750.6;
const GLUE_ROW_BASE: f32 = 510.3;

/// Anti-aliased fill of closed polygons (even-odd, 4x4 supersampling) in
/// window pixels.
fn aa_polygons(fb: &mut Framebuffer, polys: &[&[(f32, f32)]], color: Rgb) {
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for p in polys {
        for &(x, y) in p.iter() {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
    }
    let inside = |px: f32, py: f32| {
        let mut odd = false;
        for p in polys {
            let n = p.len();
            for i in 0..n {
                let (ax, ay) = p[i];
                let (bx, by) = p[(i + 1) % n];
                if (ay > py) != (by > py) && px < ax + (py - ay) * (bx - ax) / (by - ay) {
                    odd = !odd;
                }
            }
        }
        odd
    };
    for y in y0.floor().max(0.0) as i32..(y1.ceil() as i32).min(fb.height) {
        for x in x0.floor().max(0.0) as i32..(x1.ceil() as i32).min(fb.width) {
            let mut n = 0;
            for j in 0..4 {
                for i in 0..4 {
                    if inside(
                        x as f32 + (i as f32 + 0.5) / 4.0,
                        y as f32 + (j as f32 + 0.5) / 4.0,
                    ) {
                        n += 1;
                    }
                }
            }
            if n > 0 {
                fb.put(
                    x,
                    y,
                    fb.get(x, y)
                        .blend(color, (n as f32 / 16.0 * 255.0).round() as u8),
                );
            }
        }
    }
}

/// Mission-list scrollbar, measured on the live client (content px):
/// #6b0000 track box 1771..1818.8 x 516..876 (2.8 px), #e40000 arrow
/// outlines (apex 461.3 / 930.6, 3 px base bars) and a #ff0000 octagonal
/// thumb 1773.4..1815.9 x 42 (3.3 px stroke, 8.7 px chamfer) whose top
/// runs from 519 to 834.
fn draw_mission_scrollbar(fb: &mut Framebuffer, cx: i32, k: f32, thumb: f32) {
    let p = |x: f32, y: f32| (cx as f32 + x * k, y * k);
    let space = CefSpace {
        cx: cx as f32,
        kx: k,
        ky: k,
    };
    cef_band(
        fb,
        space,
        [1771.0, 516.0, 1818.8, 876.0],
        0.0,
        2.9,
        Rgb::new(107, 0, 0),
        1.0,
        &[],
    );
    let arrow = Rgb::new(228, 0, 0);
    let (l, r, mid, dx) = (1770.6f32, 1818.6f32, 1794.5f32, 0.491f32);
    // Up: outer apex 461.3, base 510; inner apex 471.3, inner base 507
    // (the 4.9 px horizontal stroke runs parallel to the outer edges).
    let up_o = [p(mid, 461.3), p(r, 510.0), p(l, 510.0)];
    let inner_half = mid - (1794.5 - dx * (507.0 - 461.3) + 4.9);
    let up_i = [
        p(mid, 471.3),
        p(mid + inner_half, 507.0),
        p(mid - inner_half, 507.0),
    ];
    aa_polygons(fb, &[&up_o, &up_i], arrow);
    // Down: outer base 881.9, apex 930.6; inner base 885, apex 920.6.
    let dn_half = mid - (1794.5 - dx * (930.6 - 885.0) + 4.9);
    let dn_o = [p(l, 881.9), p(r, 881.9), p(mid, 930.6)];
    let dn_i = [
        p(mid - dn_half, 885.0),
        p(mid + dn_half, 885.0),
        p(mid, 920.6),
    ];
    aa_polygons(fb, &[&dn_o, &dn_i], arrow);
    // Thumb.
    let top = galaxycraft::menu::MISSION_THUMB_TOP.0
        + (galaxycraft::menu::MISSION_THUMB_TOP.1 - galaxycraft::menu::MISSION_THUMB_TOP.0) * thumb;
    let oct = |x0: f32, y0: f32, x1: f32, y1: f32, c: f32| {
        [
            p(x0 + c, y0),
            p(x1 - c, y0),
            p(x1, y0 + c),
            p(x1, y1 - c),
            p(x1 - c, y1),
            p(x0 + c, y1),
            p(x0, y1 - c),
            p(x0, y0 + c),
        ]
    };
    let t = 3.3f32;
    let outer = oct(1773.4, top, 1815.9, top + 42.0, 8.7);
    let inner = oct(
        1773.4 + t,
        top + 3.0,
        1815.9 - t,
        top + 39.0,
        8.7 - t * 0.414,
    );
    aa_polygons(fb, &[&outer, &inner], Rgb::new(255, 0, 0));
}

/// Retail Options plate (130933..130954), content units: an L-shaped
/// frame whose title strip and five tab boxes are separate rounded boxes
/// (4.6 px #331f1f outer band r 3.5, 3 px #6c0000 inner band r 4, 7-9 px
/// gaps left unfilled), a divider (brown 837.2..841.9, red 844.9..847.9)
/// running to the plate bottom, and the content column outline with
/// 9.5/7.5 px rounded right corners.  The selected tab opens into the
/// content column: both divider bands break and its rules run through.
fn draw_options_plate(fb: &mut Framebuffer, s: CefSpace, rows: &[f32; 5], active: usize) {
    const TB: f32 = 4.6;
    const TR: f32 = 3.0;
    const FILL: f32 = 230.0 / 255.0;
    let black = Rgb::new(0, 0, 0);
    let cell = |b: f32| [513.1, b + 16.5, 841.9, b + 103.5];
    // Fills: content column + title strip, then each tab box.
    cef_band(
        fb,
        s,
        [500.0, 172.4, 2049.9, 1152.0],
        9.5,
        0.0,
        black,
        FILL,
        &[[841.9, 0.0, 3000.0, 1500.0]],
    );
    cef_band(
        fb,
        s,
        [513.1, 172.4, 841.9, 355.5],
        4.5,
        0.0,
        black,
        FILL,
        &[],
    );
    for b in rows {
        cef_band(fb, s, cell(*b), 4.5, 0.0, black, FILL, &[]);
    }
    // Red inner bands.
    cef_band(
        fb,
        s,
        [520.8, 180.0, 2042.25, 1144.5],
        10.0,
        TR,
        CSS_INNER,
        1.0,
        &[[848.9, 0.0, 3000.0, 190.0], [844.9, 190.0, 3000.0, 1500.0]],
    );
    cef_band(
        fb,
        s,
        [520.8, 180.0, 851.9, 348.0],
        5.0,
        TR,
        CSS_INNER,
        1.0,
        &[
            [0.0, 0.0, 530.0, 400.0],
            [0.0, 180.0, 847.9, 183.5],
            [0.0, 344.5, 847.9, 400.0],
        ],
    );
    for (i, b) in rows.iter().enumerate() {
        if i == active {
            cef_band(
                fb,
                s,
                [520.8, b + 24.0, 851.9, b + 96.0],
                5.0,
                TR,
                CSS_INNER,
                1.0,
                &[
                    [0.0, 0.0, 844.5, 2000.0],
                    [0.0, 0.0, 847.9, b + 27.5],
                    [0.0, b + 92.5, 847.9, 2000.0],
                ],
            );
        } else {
            cef_band(
                fb,
                s,
                [520.8, b + 24.0, 841.9, b + 96.0],
                5.0,
                TR,
                CSS_INNER,
                1.0,
                &[],
            );
        }
    }
    let ab = rows[active];
    for (y0, y1) in [(349.5, ab + 28.0), (ab + 98.0, 1144.5)] {
        if y1 > y0 {
            cef_band(fb, s, [844.9, y0, 847.9, y1], 0.0, 0.0, CSS_INNER, 1.0, &[]);
        }
    }
    // Brown outer bands.
    cef_band(
        fb,
        s,
        [513.1, 172.4, 2049.9, 1152.0],
        9.5,
        TB,
        CSS_OUTER,
        1.0,
        &[[530.0, 0.0, 3000.0, 177.5], [837.2, 177.5, 3000.0, 1500.0]],
    );
    cef_band(
        fb,
        s,
        [513.1, 172.4, 841.9, 355.5],
        4.5,
        TB,
        CSS_OUTER,
        1.0,
        &[[0.0, 0.0, 530.0, 400.0], [0.0, 350.5, 841.9, 400.0]],
    );
    for (i, b) in rows.iter().enumerate() {
        if i == active {
            let mut c = cell(*b);
            c[2] = 845.9;
            cef_band(
                fb,
                s,
                c,
                4.5,
                TB,
                CSS_OUTER,
                1.0,
                &[
                    [0.0, 0.0, 837.2, 2000.0],
                    [0.0, 0.0, 841.9, b + 21.1],
                    [0.0, b + 98.9, 841.9, 2000.0],
                ],
            );
        } else {
            cef_band(fb, s, cell(*b), 4.5, TB, CSS_OUTER, 1.0, &[]);
        }
    }
    for (y0, y1) in [(349.55, ab + 21.1), (ab + 98.0, 1152.0)] {
        if y1 > y0 {
            cef_band(fb, s, [837.2, y0, 841.9, y1], 0.0, 0.0, CSS_OUTER, 1.0, &[]);
        }
    }
}

/// Columns `[a, b)` of a `rounded_sdf` row where the distance is exactly
/// `-r` (the row is outside the corner bands and the pixel centre is inside
/// the straight part): `None` for rows that cross a corner.  Lets the
/// rounded primitives fill the long middle runs without per-pixel maths.
fn rounded_flat_span(yy: i32, x: i32, y: i32, w: i32, h: i32, r: f32) -> Option<(i32, i32)> {
    let (rx, ry) = (w as f32 / 2.0, h as f32 / 2.0);
    let r = r.min(rx).min(ry).max(0.0);
    let qy = ((yy as f32 + 0.5 - (y as f32 + ry)).abs() - (ry - r)).max(0.0);
    if qy > 0.0 {
        return None;
    }
    // qx == 0 for |px - cx| <= rx - r, with px the pixel centre.
    let m = r.ceil() as i32 + 1;
    (w > 2 * m).then_some((x + m, x + w - m))
}

/// Opaque rounded fill (the CEF dialog plates have a 16 px radius at 3x).
fn rounded_fill(fb: &mut Framebuffer, x: i32, y: i32, w: i32, h: i32, r: f32, color: Rgb) {
    for yy in y..y + h {
        let flat = rounded_flat_span(yy, x, y, w, h, r);
        if let Some((a, b)) = flat {
            // d = -r <= 0 on the whole span.
            fb.rect(a, yy, b - a, 1, color);
        }
        for xx in x..x + w {
            if flat.is_some_and(|(a, b)| xx >= a && xx < b) {
                continue;
            }
            if rounded_sdf(
                xx as f32 + 0.5,
                yy as f32 + 0.5,
                x as f32,
                y as f32,
                w as f32,
                h as f32,
                r,
            ) <= 0.0
            {
                fb.put(xx, yy, color);
            }
        }
    }
}

/// Rounded border band inside a rounded rect at `inset`.
#[allow(clippy::too_many_arguments)]
fn rounded_band(
    fb: &mut Framebuffer,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    r: f32,
    inset: i32,
    t: i32,
    color: Rgb,
) {
    let (ix, iy, iw, ih) = (x + inset, y + inset, w - 2 * inset, h - 2 * inset);
    if iw <= 0 || ih <= 0 {
        return;
    }
    let rb = (r - inset as f32).max(0.0);
    for yy in iy..iy + ih {
        let flat = rounded_flat_span(yy, ix, iy, iw, ih, rb);
        if let Some((a, b)) = flat {
            // d = -r on the span: painted only when the band reaches it.
            let d = -rb.min(iw as f32 / 2.0).min(ih as f32 / 2.0);
            if d > -(t as f32) && d <= 0.0 {
                fb.rect(a, yy, b - a, 1, color);
            }
        }
        for xx in ix..ix + iw {
            if flat.is_some_and(|(a, b)| xx >= a && xx < b) {
                continue;
            }
            let d = rounded_sdf(
                xx as f32 + 0.5,
                yy as f32 + 0.5,
                ix as f32,
                iy as f32,
                iw as f32,
                ih as f32,
                rb,
            );
            if d > -(t as f32) && d <= 0.0 {
                fb.put(xx, yy, color);
            }
        }
    }
}

/// Black fill with alpha, clipped to a rounded rectangle.
fn rounded_fill_alpha(fb: &mut Framebuffer, x: i32, y: i32, w: i32, h: i32, r: f32, alpha: u8) {
    let black = Rgb::new(0, 0, 0);
    for yy in y..y + h {
        let flat = rounded_flat_span(yy, x, y, w, h, r);
        if let Some((a, b)) = flat {
            let re = r.min(w as f32 / 2.0).min(h as f32 / 2.0).max(0.0);
            let cov = (0.5 + re).clamp(0.0, 1.0);
            let k = (alpha as f32 * cov) as u8;
            if yy >= 0 && yy < fb.height {
                let stride = fb.width as usize;
                let row = &mut fb.pixels[yy as usize * stride..(yy as usize + 1) * stride];
                for p in &mut row[a.clamp(0, fb.width) as usize..b.clamp(0, fb.width) as usize] {
                    *p = Rgb(*p).blend(black, k).0;
                }
            }
        }
        for xx in x..x + w {
            if flat.is_some_and(|(a, b)| xx >= a && xx < b) {
                continue;
            }
            let sdf = rounded_sdf(
                xx as f32 + 0.5,
                yy as f32 + 0.5,
                x as f32,
                y as f32,
                w as f32,
                h as f32,
                r,
            );
            let cov = (0.5 - sdf).clamp(0.0, 1.0);
            if cov > 0.0 {
                let c = fb.get(xx, yy);
                fb.put(
                    xx,
                    yy,
                    c.blend(Rgb::new(0, 0, 0), (alpha as f32 * cov) as u8),
                );
            }
        }
    }
}

/// Alpha fill whose top corners are rounded and whose bottom edge is open
/// (tabs sit on the panel top border).
fn top_rounded_fill(fb: &mut Framebuffer, x: i32, y: i32, w: i32, h: i32, r: f32, alpha: u8) {
    for yy in 0..h {
        for xx in 0..w {
            let mut cov = 1.0f32;
            if yy < r as i32 {
                let cx = if xx < r as i32 {
                    x as f32 + r
                } else if xx >= w - r as i32 {
                    x as f32 + w as f32 - r
                } else {
                    x as f32 + xx as f32 + 0.5
                };
                let cy = y as f32 + r;
                let d = (((x + xx) as f32 + 0.5 - cx).powi(2)
                    + ((y + yy) as f32 + 0.5 - cy).powi(2))
                .sqrt();
                cov = (r + 0.5 - d).clamp(0.0, 1.0);
            }
            if cov > 0.0 {
                let c = fb.get(x + xx, y + yy);
                fb.put(
                    x + xx,
                    y + yy,
                    c.blend(Rgb::new(0, 0, 0), (alpha as f32 * cov) as u8),
                );
            }
        }
    }
}

/// CEF tab border: rounded top corners, straight sides that may extend below
/// the tab (selected tab `margin-bottom:-3px`), and no bottom border.
fn tab_border_rounded(
    fb: &mut Framebuffer,
    r: (i32, i32, i32, i32),
    t: i32,
    inset: i32,
    radius: i32,
    extend: i32,
    color: Rgb,
) {
    let (x, y, w, h) = r;
    let bottom = y + h + extend;
    // Straight top and side segments (the corner boxes are painted by arcs).
    let rad = radius.max(t + 1);
    fb.rect(x + rad, y, w - 2 * rad, t, color);
    fb.rect(x, y + rad, t, bottom - y - rad, color);
    fb.rect(x + w - t, y + rad, t, bottom - y - rad, color);
    let band = t as f32;
    let rr = rad as f32;
    for (cx, cy, sx, sy) in [
        (x + rad, y + rad, x, y),
        (x + w - rad, y + rad, x + w - rad, y),
    ] {
        for yy in sy..sy + rad {
            for xx in sx..sx + rad {
                let d = (((xx as f32 + 0.5) - cx as f32).powi(2)
                    + ((yy as f32 + 0.5) - cy as f32).powi(2))
                .sqrt();
                let cov = (d - (rr - band) + 0.5).clamp(0.0, 1.0) * (rr - d + 0.5).clamp(0.0, 1.0);
                if cov > 0.0 {
                    fb.put(xx, yy, fb.get(xx, yy).blend(color, (cov * 255.0) as u8));
                }
            }
        }
    }
    let _ = inset; // the caller passes the color for each concentric band
}

/// Full panel plate with rounded outer/inner bands.
fn css_panel3_rounded(
    fb: &mut Framebuffer,
    r: (i32, i32, i32, i32),
    alpha: u8,
    t: i32,
    inset: i32,
    radius: i32,
) {
    let (x, y, w, h) = r;
    rounded_fill_alpha(fb, x, y, w, h, radius as f32, alpha);
    rounded_band(fb, x, y, w, h, radius as f32, 0, t, CSS_OUTER);
    rounded_band(fb, x, y, w, h, radius as f32, inset, t, CSS_INNER);
}

/// Retail CEF button plate: transparent black, #6c0000 border, green text.
fn dim_screen(fb: &mut Framebuffer, cx: i32, cw: i32, ch: i32, alpha: u8) {
    let black = Rgb::new(0, 0, 0);
    let (x0, x1) = (cx.max(0), (cx + cw).min(fb.width));
    let stride = fb.width as usize;
    for y in 0..ch.min(fb.height).max(0) {
        let row = &mut fb.pixels[y as usize * stride..(y as usize + 1) * stride];
        for p in &mut row[x0.max(0) as usize..x1.max(x0) as usize] {
            *p = Rgb(*p).blend(black, alpha).0;
        }
    }
}

/// A movie fills the 16:9 content box, as the retail client draws every
/// film to its full viewport: the 88 HD films are 1920x1080 already, and the
/// SD `smk/blizzard.webm` (320x308) is stretched to the box instead of being
/// shown with square pixels, which squeezed the logo (user report on the
/// retail client, 2026-11-04; INFERENCE without a retail capture).
fn draw_movie_frame(fb: &mut Framebuffer, frame: &PosterFrame) {
    if frame.width == 0 || frame.height == 0 {
        return;
    }
    fb.clear(Rgb::new(0, 0, 0));
    let (ox, oy, dw, dh) = App::content_rect(fb);
    let (kx, ky) = (
        dw as f32 / frame.width as f32,
        dh as f32 / frame.height as f32,
    );
    for y in 0..dh {
        let sy = ((y as f32 / ky) as usize).min(frame.height as usize - 1);
        for x in 0..dw {
            let sx = ((x as f32 / kx) as usize).min(frame.width as usize - 1);
            let o = (sy * frame.width as usize + sx) * 3;
            fb.put(
                ox + x,
                oy + y,
                Rgb::new(frame.rgb[o], frame.rgb[o + 1], frame.rgb[o + 2]),
            );
        }
    }
}

/// Colour marking "no world pixel" in the 1x unit layer composited over the
/// screen-resolution HD terrain.
const WORLD_KEY: Rgb = Rgb(0x0001_0203);

/// Bilinear blit of the 1x layer, skipping [`WORLD_KEY`] texels: each output
/// pixel blends the non-key neighbours by their bilinear weight, so sprite
/// edges stay as soft as the opaque blit.
fn blit_scaled_keyed(src: &Framebuffer, dst: &mut Framebuffer, scale: f32, x0: i32) {
    // Bilinear upscale of the 1x world layer that skips `WORLD_KEY` texels.
    // In the HD path that layer is almost all key (the HD terrain and units
    // are drawn at screen resolution), so source rows without a visible
    // texel are skipped outright and the horizontal taps are computed once
    // per column (it was 20 ms of the 1080p frame).
    let s = scale.max(0.5);
    let dw = ((src.width as f32 * s).round() as i32).min(dst.width - x0);
    let dh = ((src.height as f32 * s).round() as i32).min(dst.height);
    if dw <= 0 || dh <= 0 {
        return;
    }
    let (sw, sh) = (src.width as usize, src.height as usize);
    let row_visible: Vec<bool> = (0..sh)
        .map(|y| {
            src.pixels[y * sw..(y + 1) * sw]
                .iter()
                .any(|&p| p != WORLD_KEY.0)
        })
        .collect();
    if !row_visible.iter().any(|&v| v) {
        return;
    }
    let max_sx = src.width - 1;
    let max_sy = src.height - 1;
    let taps_x: Vec<(usize, usize, f32)> = (0..dw)
        .map(|x| {
            let fx = ((x as f32 + 0.5) / s) - 0.5;
            let xs = fx.floor() as i32;
            let wx = fx - xs as f32;
            (
                xs.clamp(0, max_sx) as usize,
                (xs + 1).clamp(0, max_sx) as usize,
                wx,
            )
        })
        .collect();
    let dst_w = dst.width as usize;
    for y in 0..dh {
        let fy = ((y as f32 + 0.5) / s) - 0.5;
        let y0 = fy.floor() as i32;
        let wy = fy - y0 as f32;
        let (ya, yb) = (
            y0.clamp(0, max_sy) as usize,
            (y0 + 1).clamp(0, max_sy) as usize,
        );
        if !row_visible[ya] && !row_visible[yb] {
            continue;
        }
        let (ra, rb) = (
            &src.pixels[ya * sw..(ya + 1) * sw],
            &src.pixels[yb * sw..(yb + 1) * sw],
        );
        let out_row = &mut dst.pixels[y as usize * dst_w..(y as usize + 1) * dst_w];
        for (x, &(xa, xb, wx)) in taps_x.iter().enumerate() {
            let taps = [
                (ra[xa], (1.0 - wx) * (1.0 - wy)),
                (ra[xb], wx * (1.0 - wy)),
                (rb[xa], (1.0 - wx) * wy),
                (rb[xb], wx * wy),
            ];
            if taps.iter().all(|(p, _)| *p == WORLD_KEY.0) {
                continue;
            }
            let (mut r, mut g, mut b, mut wsum) = (0f32, 0f32, 0f32, 0f32);
            for (p, w) in taps {
                if p == WORLD_KEY.0 || w <= 0.0 {
                    continue;
                }
                r += ((p >> 16) & 0xff) as f32 * w;
                g += ((p >> 8) & 0xff) as f32 * w;
                b += (p & 0xff) as f32 * w;
                wsum += w;
            }
            if wsum <= 0.0 {
                continue;
            }
            let c = Rgb::new(
                (r / wsum).round() as u8,
                (g / wsum).round() as u8,
                (b / wsum).round() as u8,
            );
            let px = (x0 + x as i32) as usize;
            if px >= dst_w {
                continue;
            }
            let a = (wsum * 255.0).round().min(255.0) as u8;
            out_row[px] = if a == 255 {
                c.0
            } else {
                Rgb(out_row[px]).blend(c, a).0
            };
        }
    }
}

fn blit_scaled_at(src: &Framebuffer, dst: &mut Framebuffer, scale: f32, x0: i32, filter: f32) {
    let s = scale.max(0.5);
    let dw = ((src.width as f32 * s).round() as i32).min(dst.width);
    let dh = ((src.height as f32 * s).round() as i32).min(dst.height);
    let max_sx = (src.width - 1) as f32;
    let max_sy = (src.height - 1) as f32;
    // The Video tab's SD Graphics Filter: the sharp end is nearest-neighbour
    // (the classic 640x480 look), while the filtered stops use the bilinear
    // upscale measured on the retail HD screenshots.
    let sharp = filter <= 0.01;
    if sharp {
        for y in 0..dh {
            let sy = (y as f32 / s).floor().clamp(0.0, max_sy) as i32;
            for x in 0..dw {
                let sx = (x as f32 / s).floor().clamp(0.0, max_sx) as i32;
                dst.put(x0 + x, y, src.get(sx, sy));
            }
        }
        return;
    }
    // Bilinear, like the retail client's HD upscale: the SD art is magnified
    // smoothly there, while a nearest-neighbour blit leaves hard 3 px steps
    // that the references do not have.
    for y in 0..dh {
        let fy = (((y as f32 + 0.5) / s) - 0.5).clamp(0.0, max_sy);
        let y0 = fy.floor() as i32;
        let y1 = (y0 + 1).min(max_sy as i32);
        let wy = fy - y0 as f32;
        for x in 0..dw {
            let fx = (((x as f32 + 0.5) / s) - 0.5).clamp(0.0, max_sx);
            let x0s = fx.floor() as i32;
            let x1s = (x0s + 1).min(max_sx as i32);
            let wx = fx - x0s as f32;
            let p00 = src.get(x0s, y0).0;
            let p10 = src.get(x1s, y0).0;
            let p01 = src.get(x0s, y1).0;
            let p11 = src.get(x1s, y1).0;
            let mix = |a: u32, b: u32, t: f32| -> u32 {
                let a = a as f32;
                let b = b as f32;
                (a + (b - a) * t).round() as u32
            };
            let top = mix(p00 & 0xff, p10 & 0xff, wx);
            let bottom = mix(p01 & 0xff, p11 & 0xff, wx);
            let blue = mix(top, bottom, wy);
            let top = mix((p00 >> 8) & 0xff, (p10 >> 8) & 0xff, wx);
            let bottom = mix((p01 >> 8) & 0xff, (p11 >> 8) & 0xff, wx);
            let green = mix(top, bottom, wy);
            let top = mix(p00 >> 16, p10 >> 16, wx);
            let bottom = mix(p01 >> 16, p11 >> 16, wx);
            let red = mix(top, bottom, wy);
            dst.put(
                x0 + x,
                y,
                galaxycraft::render::Rgb::new(red as u8, green as u8, blue as u8),
            );
        }
    }
}

fn parse_race(s: &str) -> Race {
    match s.to_ascii_lowercase().as_str() {
        "zerg" | "z" => Race::Zerg,
        "protoss" | "p" => Race::Protoss,
        _ => Race::Terran,
    }
}

/// Race per player slot.  Campaign maps keep their authored SIDE races;
/// melee maps use the player's chosen race for every slot, matching the
/// retail lobby where the map side is only a default.  An explicit
/// `--races` list overrides both and is cycled across the slots.
fn races_for(
    scenario: &Scenario,
    slots: &[PlayerSlot],
    override_races: &Option<Vec<Race>>,
    menu_race: Race,
) -> Vec<Race> {
    match override_races {
        Some(list) if !list.is_empty() => (0..slots.len()).map(|i| list[i % list.len()]).collect(),
        _ if scenario.has_placed_force() => slots.iter().map(|s| s.race(menu_race)).collect(),
        _ => vec![menu_race; slots.len()],
    }
}

/// Apply the recovered slot control types to a fresh world: inactive slots
/// never count toward victory, and computer slots are driven by the AI.
fn apply_slots(world: &mut World, slots: &[PlayerSlot]) {
    for s in slots.iter() {
        if let Some(p) = world.players.get_mut(s.slot) {
            p.defeated = !s.active;
            p.is_computer = s.computer;
        }
    }
}

/// Mission text authored in the scenario's `STR` section (see
/// [`Scenario::briefing`]), flattened for the briefing screen.
fn briefing_content(scenario: &Scenario) -> Option<(String, String, String)> {
    scenario.briefing().map(|b| (b.name, b.text, b.objectives))
}

/// Word-wrap fixed-width text, preserving authored line breaks.
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let mut current = String::new();
        for word in line.split_whitespace() {
            if !current.is_empty() && current.len() + 1 + word.len() > width {
                out.push(std::mem::take(&mut current));
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
        out.push(current);
    }
    out
}

fn format_time(ticks: u64) -> String {
    let seconds = ticks / 24;
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

/// Map a script key name to a minifb key.
fn script_key(name: &str) -> Key {
    match name {
        "Up" => Key::Up,
        "Down" => Key::Down,
        "Left" => Key::Left,
        "Right" => Key::Right,
        "Return" | "Enter" => Key::Enter,
        "Escape" | "Esc" => Key::Escape,
        "Tab" => Key::Tab,
        "Space" => Key::Space,
        "Delete" => Key::Delete,
        "L" => Key::L,
        "R" => Key::R,
        "H" => Key::H,
        "S" => Key::S,
        "C" => Key::C,
        "V" => Key::V,
        "U" => Key::U,
        "T" => Key::T,
        "O" => Key::O,
        "F10" => Key::F10,
        "Home" => Key::Home,
        name if name.len() == 1 => {
            const LETTERS: [Key; 26] = [
                Key::A,
                Key::B,
                Key::C,
                Key::D,
                Key::E,
                Key::F,
                Key::G,
                Key::H,
                Key::I,
                Key::J,
                Key::K,
                Key::L,
                Key::M,
                Key::N,
                Key::O,
                Key::P,
                Key::Q,
                Key::R,
                Key::S,
                Key::T,
                Key::U,
                Key::V,
                Key::W,
                Key::X,
                Key::Y,
                Key::Z,
            ];
            const DIGITS: [Key; 10] = [
                Key::Key0,
                Key::Key1,
                Key::Key2,
                Key::Key3,
                Key::Key4,
                Key::Key5,
                Key::Key6,
                Key::Key7,
                Key::Key8,
                Key::Key9,
            ];
            let c = name.as_bytes()[0];
            match c {
                b'A'..=b'Z' => LETTERS[(c - b'A') as usize],
                b'0'..=b'9' => DIGITS[(c - b'0') as usize],
                _ => Key::Unknown,
            }
        }
        _ => Key::Unknown,
    }
}

/// A pseudo-random voice take (presentation only: seeded from the clock and
/// the previous value, never from the simulation's RNG).
fn random_take(prev: usize) -> usize {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos() as usize);
    (prev.wrapping_mul(1_103_515_245).wrapping_add(nanos) >> 7) & 0xffff
}

/// Scripted in-game session (`--game-script`, map from `--shot-map`):
/// synthetic [`GameInput`] frames through the real in-game handler, so
/// selection, orders, the command card and building are testable without
/// a window.  Steps (comma separated; points are window pixels):
/// `move:x:y`, `click:x:y`, `shift-click:x:y`, `rclick:x:y`,
/// `drag:x0:y0:x1:y1`, `drag-hold:x0:y0:x1:y1`, `hold:<arrow>:N`,
/// `spawn:OWNER:TYPE_ID:x:y`, `arm-silos`, `key:K`, `ctrl-key:K`, `ticks:N` (simulation ticks),
/// `click-unit:NAME` / `rclick-unit:NAME` (the nearest on-screen unit whose
/// name contains NAME), `click-cmd:N` (command-card slot N, 1..9),
/// `click-world:x:y` / `rclick-world:x:y` (world game px), `click-view:px:py` / `rclick-view`
/// (percent of the visible playfield), `minimap-click:px:py` /
/// `minimap-rclick` (percent of the minimap), `look:x:y` (minimap click on a
/// world point), `dump-units:NAME`, `shot:NAME`, `shot-cursor:NAME` (with the pointer).  Writes `game.tsv` (one
/// state line per step) to `GC_WALK_DIR`.
fn run_game_script(app: &mut App, fb: &mut Framebuffer, script: &str) {
    let dir = std::env::var("GC_WALK_DIR").unwrap_or_else(|_| "/tmp/opencode/game".into());
    let _ = std::fs::create_dir_all(&dir);
    app.load_map(app.map_index);
    app.briefing = false;
    app.intro = false;
    app.menu = false;
    app.update_view(fb);
    app.draw(fb);
    let mut log = String::from("step\taction\tstate\n");
    let mut cue_log = String::from("tick\tkind\tcontent\n");
    let mut cues_logged = 0usize;
    let mut mouse: Option<(i32, i32)> = None;
    let frame = |app: &mut App, fb: &mut Framebuffer, input: GameInput| {
        // The window loop sets the hover pointer every frame (tooltips).
        app.pointer = input.mouse;
        app.handle_game_input(&input);
        app.draw(fb);
    };
    for (i, step) in script.split(',').map(str::trim).enumerate() {
        let parts: Vec<&str> = step.split(':').collect();
        let num = |k: usize| {
            parts
                .get(k)
                .and_then(|v| v.parse::<i32>().ok())
                .unwrap_or(0)
        };
        let base = |mouse: Option<(i32, i32)>| GameInput {
            mouse,
            ..GameInput::default()
        };
        let click_mod = |app: &mut App,
                         fb: &mut Framebuffer,
                         at: (i32, i32),
                         right: bool,
                         shift: bool,
                         ctrl: bool| {
            let p = Some(surface_point(app.present_offset, at.0, at.1));
            let held = GameInput {
                lmb: !right,
                rmb: right,
                shift,
                ctrl,
                ..base(p)
            };
            frame(app, fb, held);
            frame(
                app,
                fb,
                GameInput {
                    shift,
                    ctrl,
                    ..base(p)
                },
            );
            p
        };
        let click =
            |app: &mut App, fb: &mut Framebuffer, at: (i32, i32), right: bool, shift: bool| {
                click_mod(app, fb, at, right, shift, false)
            };
        match parts[0] {
            "move" => {
                mouse = Some(surface_point(app.present_offset, num(1), num(2)));
                frame(app, fb, base(mouse));
            }
            "spawn" => {
                // `spawn:OWNER:TYPE_ID:x:y` (world px): a test fixture for
                // scenes the map does not start with (a skirmish to watch).
                let _ = app.world.spawn_px(
                    num(1).max(0) as usize,
                    num(2).max(0) as u16,
                    num(3),
                    num(4),
                );
                frame(app, fb, base(mouse));
            }
            "site" => {
                // `site:OWNER:TYPE:x:y:PCT`: a building under construction.
                app.spawn_site(num(1), num(2), (num(3), num(4)), num(5));
                frame(app, fb, base(mouse));
            }
            "arm-silos" => {
                // Test fixture: every Nuclear Silo is attached and holds a
                // missile (a scripted `spawn` cannot build one).
                for u in app.world.units.iter_mut() {
                    if u.type_id == galaxycraft::ids::unit::NUCLEAR_SILO {
                        u.completed = true;
                        u.addon_attached = true;
                        u.nuke_ready = true;
                    }
                }
                frame(app, fb, base(mouse));
            }
            "cheat" => {
                // `cheat:<code>`: the same prompt path as a typed cheat.
                let text = parts[1..].join(":");
                app.send_chat(text);
                frame(app, fb, base(mouse));
            }
            "hold" => {
                // `hold:<Left|Right|Up|Down>:<frames>`: an arrow key held
                // down, as the window reports it (camera scrolling).
                for _ in 0..num(2).max(1) {
                    let dir = parts.get(1).copied().unwrap_or("");
                    let held = GameInput {
                        left: dir == "Left",
                        right: dir == "Right",
                        up: dir == "Up",
                        down: dir == "Down",
                        ..base(mouse)
                    };
                    frame(app, fb, held);
                }
            }
            "click" | "shift-click" | "rclick" => {
                mouse = click(
                    app,
                    fb,
                    (num(1), num(2)),
                    parts[0] == "rclick",
                    parts[0] == "shift-click",
                );
            }
            "drag-hold" => {
                // `drag-hold:x0:y0:x1:y1`: press at the first point and move
                // to the second without releasing (the box is on screen).
                let a = Some(surface_point(app.present_offset, num(1), num(2)));
                let b = Some(surface_point(app.present_offset, num(3), num(4)));
                for p in [a, b] {
                    frame(
                        app,
                        fb,
                        GameInput {
                            lmb: true,
                            ..base(p)
                        },
                    );
                }
                mouse = b;
            }
            "drag" => {
                let a = Some(surface_point(app.present_offset, num(1), num(2)));
                let b = Some(surface_point(app.present_offset, num(3), num(4)));
                frame(
                    app,
                    fb,
                    GameInput {
                        lmb: true,
                        ..base(a)
                    },
                );
                frame(
                    app,
                    fb,
                    GameInput {
                        lmb: true,
                        ..base(b)
                    },
                );
                frame(app, fb, base(b));
                mouse = b;
            }
            "key" | "ctrl-key" | "alt-key" => {
                let input = GameInput {
                    pressed: vec![script_key(parts.get(1).copied().unwrap_or(""))],
                    ctrl: parts[0] == "ctrl-key",
                    alt: parts[0] == "alt-key",
                    ..base(mouse)
                };
                frame(app, fb, input);
            }
            "ticks" => {
                for _ in 0..num(1).max(0) {
                    app.step_once();
                    // Trigger cues with the tick they appeared (timeline
                    // comparisons against the recordings).
                    if let Some(t) = app.world.triggers.as_ref() {
                        for cue in &t.cues[cues_logged.min(t.cues.len())..] {
                            let text = match *cue {
                                galaxycraft::trigger::TriggerCue::Text { string, .. } => format!(
                                    "text\t{}",
                                    app.scenario
                                        .string(string)
                                        .unwrap_or("")
                                        .replace(['\n', '\r', '\t'], " ")
                                ),
                                galaxycraft::trigger::TriggerCue::Wav { string, .. } => {
                                    format!("wav\t{}", app.scenario.string(string).unwrap_or(""))
                                }
                            };
                            cue_log.push_str(&format!("{}\t{text}\n", app.world.tick));
                        }
                        cues_logged = t.cues.len();
                    }
                }
                app.after_steps();
                app.draw(fb);
            }
            "ctrl-click-unit" => {
                let name = parts.get(1).copied().unwrap_or("");
                if let Some(at) = app.screen_point_of_unit(name) {
                    mouse = click_mod(app, fb, at, false, false, true);
                } else {
                    eprintln!("game script: no on-screen unit named {name:?}");
                }
            }
            "click-newest" => {
                // Left click on the centre of the newest unit named NAME.
                let name = parts.get(1).copied().unwrap_or("");
                let at = app
                    .world
                    .units
                    .iter()
                    .filter(|u| u.alive && app.data.unit_name(u.type_id).contains(name))
                    .max_by_key(|u| u.id)
                    .map(|u| (u.x / 256, u.y / 256));
                if let Some((x, y)) = at {
                    let p = app.screen_point_of_world(x, y);
                    mouse = click(app, fb, p, false, false);
                }
            }
            "rclick-unit-at" => {
                // Right click on the own unit named NAME nearest world x, y.
                let name = parts.get(1).copied().unwrap_or("");
                let (wx, wy) = (num(2), num(3));
                let best = app
                    .world
                    .units
                    .iter()
                    .filter(|u| u.alive && app.data.unit_name(u.type_id).contains(name))
                    .min_by_key(|u| (u.x / 256 - wx).abs() + (u.y / 256 - wy).abs())
                    .map(|u| (u.x / 256, u.y / 256));
                if let Some((x, y)) = best {
                    let at = app.screen_point_of_world(x, y);
                    mouse = click(app, fb, at, true, false);
                }
            }
            "click-unit" | "rclick-unit" => {
                let name = parts.get(1).copied().unwrap_or("");
                if let Some(at) = app.screen_point_of_unit(name) {
                    mouse = click(app, fb, at, parts[0] == "rclick-unit", false);
                } else {
                    eprintln!("game script: no on-screen unit named {name:?}");
                }
            }
            "click-view" | "rclick-view" => {
                // Percent of the visible playfield (0..100).
                let (cx, _, cw, _) = app.content_box;
                let at = (
                    cx + cw * num(1) / 100,
                    app.playfield_bottom() * num(2) / 100,
                );
                mouse = click(app, fb, at, parts[0] == "rclick-view", false);
            }
            "dump-units" => {
                // Every live unit: id, owner, type, position, hp, order.
                let mut out = String::from("id\towner\ttype\tx\ty\thp\torder\tpath\tnext\n");
                for u in app.world.units.iter().filter(|u| u.alive) {
                    out.push_str(&format!(
                        "{}\t{}\t{}\t{}\t{}\t{}\t{:?}\t{}\t{:?}\n",
                        u.id,
                        u.owner,
                        app.data.unit_name(u.type_id),
                        u.x / 256,
                        u.y / 256,
                        u.hp / 256,
                        u.order,
                        u.path.len(),
                        u.path.front().map(|p| (p.0 / 256, p.1 / 256))
                    ));
                    if u.is_resource() {
                        out.pop();
                        out.push_str(&format!("\tres={}\n", u.resource_amount));
                    }
                    if let Some(a) = &u.anim {
                        out.pop();
                        out.push_str(&format!(
                            "\tanim={} move={} fling={} ended={} ignore={} tick={}\n",
                            a.anim,
                            a.move_speed,
                            a.fling_speed,
                            a.ended,
                            a.ignore_rest,
                            u.anim_tick
                        ));
                    }
                }
                let name = parts.get(1).copied().unwrap_or("units");
                let _ = std::fs::write(format!("{dir}/{name}.tsv"), out);
            }
            "wait-minerals" => {
                // Run frames until the local player has N minerals (at most
                // 6000): a player waits for income before ordering.
                let want = num(1);
                for _ in 0..6000 {
                    if app.world.players[app.local_player].minerals >= want {
                        break;
                    }
                    app.step_once();
                }
                app.after_steps();
                app.draw(fb);
            }
            "camera" => {
                // Test harness: put the camera's top-left at world x, y
                // (matching a recording's framing; not a player input).
                app.camera = (num(1), num(2));
                app.clamp_camera();
                app.draw(fb);
            }
            "look" => {
                // Minimap click on the world point (x, y) game px: centres
                // the camera there through the real minimap input.
                let (rx, ry, rw, rh) = app.minimap_rect();
                let (mw, mh) = app.tileset.minimap_size;
                let at = (
                    rx + (num(1) as i64 * rw as i64 / (mw.max(1) as i64 * TILE_PX as i64)) as i32,
                    ry + (num(2) as i64 * rh as i64 / (mh.max(1) as i64 * TILE_PX as i64)) as i32,
                );
                mouse = click(app, fb, at, false, false);
            }
            "minimap-click" | "minimap-rclick" => {
                // Percent of the minimap (0..100).
                let (rx, ry, rw, rh) = app.minimap_rect();
                let at = (rx + rw * num(1) / 100, ry + rh * num(2) / 100);
                mouse = click(app, fb, at, parts[0] == "minimap-rclick", false);
            }
            "click-world" | "rclick-world" => {
                let at = app.screen_point_of_world(num(1), num(2));
                mouse = click(app, fb, at, parts[0] == "rclick-world", false);
            }
            "click-cargo" => {
                // Cargo slot N of the selected transport.
                let rect = app
                    .world
                    .unit(*app.selected.first().unwrap_or(&0))
                    .and_then(|t| {
                        let ty = app.data.unit(t.type_id).clone();
                        let sizes: Vec<u8> = t
                            .cargo
                            .iter()
                            .filter_map(|id| app.world.unit(*id))
                            .map(|u| app.data.unit(u.type_id).space_required)
                            .collect();
                        let slot = cargo_slots(&sizes, ty.space_provided)
                            .get(num(1).max(0) as usize)
                            .copied()?;
                        Some(app.cargo_slot_rect(slot.control))
                    });
                if let Some(r) = rect {
                    mouse = click(app, fb, (r.x + r.w / 2, r.y + r.h / 2), false, false);
                }
            }
            "click-slot" | "shift-click-slot" => {
                // Multi-selection slot N (0..11).
                let r = app.group_slot_rect(num(1).max(0) as usize);
                mouse = click(
                    app,
                    fb,
                    (r.x + r.w / 2, r.y + r.h / 2),
                    false,
                    parts[0] == "shift-click-slot",
                );
            }
            // `pad:BUTTON` (a b x y lb rb rt lt view menu up down left
            // right) or `pad:stick:X:Y` (percent): one controller edge.
            "pad" => {
                app.pad_control = true;
                let mut f = gamepad::PadFrame {
                    active: true,
                    ..gamepad::PadFrame::default()
                };
                match parts.get(1).copied().unwrap_or("") {
                    "a" => f.primary = true,
                    "b" => f.cancel = true,
                    "x" => f.smart_order = true,
                    "y" => f.attack_move = true,
                    "lb" => f.page_prev = true,
                    "rb" => f.page_next = true,
                    "rt" => f.shortcut_radial = true,
                    "lt" => f.shift = true,
                    "view" => f.map = true,
                    "menu" => f.menu = true,
                    "left" => f.nav_pressed.0 = true,
                    "right" => f.nav_pressed.1 = true,
                    "up" => f.nav_pressed.2 = true,
                    "down" => f.nav_pressed.3 = true,
                    "stick" => f.stick = (num(2) as f32 / 100.0, num(3) as f32 / 100.0),
                    _ => {}
                }
                f.chord = !f.shift && f.stick == (0.0, 0.0);
                for pad in [
                    f,
                    gamepad::PadFrame {
                        active: true,
                        ..gamepad::PadFrame::default()
                    },
                ] {
                    frame(
                        app,
                        fb,
                        GameInput {
                            shift: pad.shift,
                            pad,
                            ..base(mouse)
                        },
                    );
                }
            }
            "click-cmd" => {
                let slots = app.command_slots();
                if let Some(r) = slots.get((num(1) - 1).max(0) as usize) {
                    mouse = click(app, fb, (r.x + r.w / 2, r.y + r.h / 2), false, false);
                }
            }
            "shot" | "shot-cursor" => {
                app.draw(fb);
                // `shot-cursor` also paints the pointer, as the window does.
                if parts[0] == "shot-cursor" {
                    app.pointer = mouse;
                    app.draw_cursor(fb);
                }
                let name = parts.get(1).copied().unwrap_or("frame");
                let _ = fb.write_bmp(std::path::Path::new(&format!("{dir}/shot-{name}.bmp")));
            }
            other => eprintln!("game script: unknown step {other:?}"),
        }
        log.push_str(&format!("{i}\t{step}\t{}\n", app.game_state_line()));
    }
    let _ = std::fs::write(format!("{dir}/cues.tsv"), &cue_log);
    if let Err(e) = std::fs::write(format!("{dir}/game.tsv"), &log) {
        eprintln!("game.tsv write failed: {e}");
    }
}

/// Scripted menu walkthrough: feeds synthetic `MenuInput` ticks through the
/// real menu handlers, writes one BMP per step and a TSV with the screen and
/// the sound cues each step produced.  `GC_WALK_DIR` overrides the output
/// directory (default `/tmp/opencode/walk`).
fn run_menu_script(app: &mut App, fb: &mut Framebuffer, script: &str) {
    let dir = std::env::var("GC_WALK_DIR").unwrap_or_else(|_| "/tmp/opencode/walk".into());
    let _ = std::fs::create_dir_all(&dir);
    let mut log = String::from("step\taction\tscreen\tsounds\tstate\n");
    let mut bank_log = String::from("cue\tloaded_samples\n");
    for cue in ["ui_move", "ui_click", "ui_swish", "ui_swish_out"] {
        let samples = app.sounds.get(cue).map(|s| s.len()).unwrap_or(0);
        bank_log.push_str(&format!("{cue}\t{samples}\n"));
    }
    if let Err(e) = std::fs::write(format!("{dir}/sounds.tsv"), &bank_log) {
        eprintln!("sounds.tsv write failed: {e}");
    }
    let mut mouse: Option<(i32, i32)> = None;
    let mut seen = app.sound_log.len();
    // One frame per screen change (plus the Lobby popup hover frame) keeps
    // the artifacts small; the TSV still records every step.
    let mut prev_screen = String::new();
    // `hold` keeps the button down across steps (drags) until `release`.
    let mut held = false;
    for (i, step) in script.split(',').map(str::trim).enumerate() {
        let mut keys = Vec::new();
        let mut down = false;
        let mut wheel = 0;
        if step == "hold" {
            held = true;
        } else if step == "release" {
            held = false;
        } else if let Some(name) = step.strip_prefix("result:") {
            // Score screen key, through the same handler as the keyboard.
            app.result_key(script_key(name));
        } else if let Some(ms) = step.strip_prefix("ready-at:") {
            // Pin the ready-room script clock and run it to that time.
            app.ready_at = ms.parse().ok();
            app.update_briefing();
        } else if let Some(name) = step.strip_prefix("briefing-key:") {
            app.briefing_key(script_key(name));
        } else if step == "intro:end" {
            // The playing movie list runs to its end (as after the last
            // film): the pending mission loads.
            app.end_intro();
        } else if step == "fixture:win" {
            // End-to-end victory: every other player is defeated in the
            // running mission and the real result path records progress.
            let me = app.local_player;
            for (k, p) in app.world.players.iter_mut().enumerate() {
                if k != me {
                    p.defeated = true;
                }
            }
            app.briefing = false;
            app.menu = false;
            app.update();
            // Menu steps do not run game frames: show the dialog now.
            app.end_dialog_hold = 0;
        } else if let Some(n) = step.strip_prefix("wheel:") {
            wheel = n.parse().unwrap_or(0);
        } else if let Some(text) = step.strip_prefix("type:") {
            // Type `text` one key at a time, Shift held for capitals (text
            // fields such as the Registry New ID popup).
            for c in text.chars() {
                if let Some(k) = char_key(c) {
                    app.handle_menu_input(&MenuInput {
                        keys: vec![k],
                        mouse,
                        mouse_down: false,
                        wheel: 0,
                        shift: c.is_ascii_uppercase(),
                    });
                }
            }
        } else if let Some(rest) = step.strip_prefix("fixture:progress:") {
            // Walkthrough fixture: give the active profile campaign history
            // (`<episode>=<next mission>`), as a played profile has.
            if let Some((ep, next)) = rest.split_once('=') {
                let campaign = ep
                    .parse()
                    .ok()
                    .and_then(galaxycraft::campaign_progress::Campaign::from_episode);
                if let (Some(c), Ok(n)) = (campaign, next.parse()) {
                    let mut p = app.progress();
                    p.next[c.index()] = n;
                    app.set_progress(p);
                }
            }
        } else if let Some(name) = step.strip_prefix("shot:") {
            // Save the frame as it is now (timed captures of animations).
            app.swish_drawn.set(None);
            app.draw_menu(fb);
            let _ = fb.write_bmp(std::path::Path::new(&format!("{dir}/shot-{name}.bmp")));
            let swish = match app.swish_drawn.get() {
                Some((true, n)) => format!("in step {n}"),
                Some((false, n)) => format!("out step {n}"),
                None => "settled".to_string(),
            };
            eprintln!("shot {name}: {swish}");
        } else if let Some(ms) = step.strip_prefix("wait-draw:") {
            // Real-time menu playback: update and draw at ~60 Hz for `ms`
            // milliseconds, as the window loop does (clip clocks advance).
            let until = Instant::now() + Duration::from_millis(ms.parse().unwrap_or(0));
            let held = MenuInput {
                keys: Vec::new(),
                mouse,
                mouse_down: false,
                wheel: 0,
                shift: false,
            };
            let (mut frames, mut draw_ms, mut worst) = (0u32, 0.0f64, 0.0f64);
            while Instant::now() < until {
                app.handle_menu_input(&held);
                app.update();
                let t0 = Instant::now();
                app.draw_menu(fb);
                let ms = t0.elapsed().as_secs_f64() * 1000.0;
                (frames, draw_ms, worst) = (frames + 1, draw_ms + ms, worst.max(ms));
                std::thread::sleep(Duration::from_millis(16));
            }
            if std::env::var_os("GC_FPS").is_some() && frames > 0 {
                eprintln!(
                    "draw {:?}: {frames} frames, mean {:.1} ms, worst {:.1} ms",
                    app.menu_screen,
                    draw_ms / frames as f64,
                    worst
                );
            }
        } else if let Some(ms) = step.strip_prefix("wait:") {
            // Real time passes (network peers can act): keep updating, as
            // the frame loop would, for `ms` milliseconds.
            let until = Instant::now() + Duration::from_millis(ms.parse().unwrap_or(0));
            while Instant::now() < until {
                app.update();
                std::thread::sleep(Duration::from_millis(10));
            }
        } else if let Some(n) = step.strip_prefix("fixture:lan-map:") {
            // Host on the first retail `(n)` melee map (campaign missions
            // count computer slots, so the name prefix is the seat count).
            let want: usize = n.parse().unwrap_or(2);
            let prefix = format!("({want})");
            if let Some(i) = (app.campaign_len..app.maps.len())
                .find(|&i| app.maps[i].name.starts_with(&prefix) && app.maps[i].players >= want)
            {
                app.lan.map_index = i;
            }
        } else if step == "fixture:lan-peer" || step.starts_with("fixture:lan-peer:") {
            // Walkthrough fixture: inject one discovered peer so the LAN row
            // action (select + join) can be exercised without a second host.
            // `fixture:lan-peer:PORT` announces a real local host on PORT
            // with the selected map's hash (end-to-end join tests).
            let port = step
                .strip_prefix("fixture:lan-peer:")
                .and_then(|p| p.parse().ok());
            let map_hash = port
                .and_then(|_| app.maps.get(app.lan.map_index))
                .and_then(|m| std::fs::read(&m.path).ok())
                .map(|raw| {
                    let mut h = [0u8; 32];
                    h.copy_from_slice(&Sha256::digest(&raw));
                    h
                })
                .unwrap_or([0u8; 32]);
            let port = port.unwrap_or(6112);
            let announce = galaxycraft::net::Announce {
                name: format!("Peer{}", app.lan.entries.len() + 1),
                map_hash,
                port,
                players: 1,
                started: false,
                game_type: galaxycraft::multiplayer::GameType::Melee.tag(),
                map_name: String::new(),
                map_size: (0, 0),
                speed: 6,
            };
            if let Ok(addr) = "127.0.0.1:6112".parse() {
                app.lan.entries.push((addr, announce));
            }
        } else if let Some(rest) = step.strip_prefix("move:") {
            let (x, y) = rest.split_once(':').unwrap_or(("0", "0"));
            // Window pixels, like a real pointer.
            mouse = Some(surface_point(
                app.present_offset,
                x.parse().unwrap_or(0),
                y.parse().unwrap_or(0),
            ));
        } else if step == "click" {
            down = true;
        } else if let Some(k) = step.strip_prefix("key:") {
            keys.push(script_key(k));
        }
        let shift = keys.contains(&Key::LeftShift);
        let input = MenuInput {
            keys,
            mouse,
            mouse_down: down || held,
            wheel,
            shift,
        };
        app.update_view(fb);
        app.handle_menu_input(&input);
        app.update_view(fb);
        app.draw_menu(fb);
        let screen = format!("{:?}", app.menu_screen)
            .split([' ', '{'])
            .next()
            .unwrap_or("?")
            .to_string();
        if screen != prev_screen
            || step == "move:1122:1039"
            || screen == "Hotkeys"
            || screen == "Lan"
        {
            let path = format!("{dir}/walk-{i:02}-{screen}.bmp");
            let _ = fb.write_bmp(std::path::Path::new(&path));
        }
        prev_screen = screen.clone();
        let sounds: Vec<String> = app.sound_log[seen..].to_vec();
        seen = app.sound_log.len();
        let o = &app.game_options;
        let bit = |v: bool| if v { '1' } else { '0' };
        let flags: String = [
            o.video_fps_cap,
            o.video_vsync,
            o.video_hd,
            o.video_environment,
            o.video_lighting,
            o.video_smooth_turning,
            o.video_shadow_stacking,
            o.video_pillarbox,
            o.game_timer,
            o.game_color_cycling,
            o.game_language_filter,
            o.game_bonus_skins,
            o.game_friend_notifications,
            o.portraits == 1,
            o.portraits == 2,
            o.minimap_position == 1,
            o.network_prefer_6112,
            o.unit_speech,
            o.unit_acks,
            o.sound_background,
            o.building_sounds,
            o.subtitles,
            o.cinematic_subtitles,
            o.original_voice_overs,
            o.original_1998_campaign,
            o.use_mouse_sensitivity,
            o.mouse_scaling,
            o.hardware_cursor,
        ]
        .iter()
        .map(|v| bit(*v))
        .collect();
        let state = match &app.menu_screen {
            MenuScreen::Options { tab } => format!(
                "tab={tab:?} drop={:?} display={} language={} speed={:.2} key={:.2} mouse={:.2} sens={:.2} music={:.2} sfx={:.2} flags={flags}",
                app.options_drop,
                app.display_mode,
                app.language,
                o.game_speed,
                o.key_scroll,
                o.mouse_scroll,
                o.mouse_sensitivity,
                o.music_volume,
                o.sfx_volume
            ),
            MenuScreen::Lobby { .. } => format!("races={:?}", app.lobby_races),
            MenuScreen::Connection
            | MenuScreen::Games
            | MenuScreen::MpCreate
            | MenuScreen::MpLobby => app.mp_state_line(),
            MenuScreen::Registry => format!(
                "ids={:?} sel={} active={}",
                app.profile_store.names(),
                app.profile_sel,
                app.active_name()
            ),
            MenuScreen::Hotkeys { race } => format!(
                "race={race:?} sel={:?} action={:?} key={:?} reset={} error={}",
                app.hotkey_sel,
                app.hotkey_action,
                app.hotkey_action.and_then(|a| {
                    app.hotkey_bindings
                        .get(a as usize)
                        .copied()
                        .flatten()
                        .map(|k| k.text)
                }),
                app.hotkey_reset,
                app.hotkey_binding_error
            ),
            MenuScreen::Mission { episode } => format!(
                "episode={episode} len={} sel={} top={} profile={} next={:?} result={:?}",
                app.mission_list.len,
                app.mission_list.sel,
                app.mission_list.top,
                app.active_name(),
                app.progress().next,
                app.game_result
            ),
            MenuScreen::Episode => format!(
                "hover={:?} prompt={:?} profile={}",
                app.episode_hover,
                app.campaign_prompt,
                app.active_name()
            ),
            MenuScreen::Lan => format!(
                "lan_sel={} entries={} lobby={} net={} map={} status={}",
                app.lan.sel,
                app.lan.entries.len(),
                app.lan
                    .queue
                    .as_ref()
                    .map(|q| format!("{}/{}", q.players(), q.capacity()))
                    .unwrap_or_default(),
                app.net
                    .as_ref()
                    .map(|n| format!(
                        "turn:{}/peers:{}/waiting:{:?}/reconnect:{:?}/last:{}",
                        n.lock.turn,
                        n.lock.peers,
                        n.waiting.map(|w| w.0),
                        n.reconnect.as_ref().map(|b| b.failures()),
                        app.status
                    ))
                    .unwrap_or_default(),
                // File name only: walk logs are committed gate outputs and
                // must not carry the machine's cache path.
                app.maps
                    .get(app.lan.map_index)
                    .and_then(|m| m.path.file_name())
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                app.lan.status
            ),
            _ => String::new(),
        };
        // Outside the front end the step records the campaign flow.
        let state = if app.menu {
            state
        } else {
            format!("{state} {}", app.flow_state()).trim().to_string()
        };
        log.push_str(&format!(
            "{i}\t{step}\t{screen}\t{}\t{state}\n",
            sounds.join("+")
        ));
    }
    if let Err(e) = std::fs::write(format!("{dir}/walk.tsv"), &log) {
        eprintln!("walk.tsv write failed: {e}");
    }
    print!("{log}");
    app.flush_profiles();
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut chk: Option<PathBuf> = None;
    let mut maps_dir = asset_root().join("maps");
    let mut tiles_dir = asset_root().join("TileSet");
    let mut assets = asset_root();
    let mut races: Option<Vec<Race>> = None;
    let mut install: Option<PathBuf> = None;
    let mut cache: Option<PathBuf> = None;
    let mut width = VIEW_W;
    let mut height = VIEW_H;
    let mut shot: Option<PathBuf> = None;
    let mut shot_screen = String::from("menu");
    let mut menu_script: Option<String> = None;
    let mut game_script: Option<String> = None;
    let mut start_screen: Option<String> = None;
    let mut freeze = false;
    // The interactive client starts fullscreen (user request 2026-11-07);
    // `--windowed` keeps a resizable window, F11 / Alt+Enter toggle.
    let mut fullscreen = true;
    // `--display wayland|x11` (or `GALAXYCRAFT_DISPLAY`): the display
    // backend; by default Wayland when the session has one, else X11.
    let mut display_backend = std::env::var("GALAXYCRAFT_DISPLAY").ok();
    let mut debug_input = false;
    let mut shot_map: Option<usize> = None;
    // Deterministic menu animation captures: `--menu-at` seeks every clip
    // (idle and hover) to the same wall-clock second.
    let mut menu_at: Option<f32> = None;
    // Per-layer overrides: `--menu-phase single=2.5,multion=6.0`.
    let mut menu_phases: std::collections::HashMap<String, f32> = std::collections::HashMap::new();
    // `--replay FILE`: open a `.gcrep`/`.gcsave` straight into the player
    // (the Replays menu does the same, this is the scriptable entry point).
    let mut replay_path: Option<PathBuf> = None;
    // `--import-characters DIR`: add the Registry IDs (and campaign progress)
    // of a retail `Characters` directory; the directory is only read.
    let mut import_characters: Option<PathBuf> = None;
    let mut no_intro = false;
    let mut no_fog = false;
    // `--hud-stats`: force Game Timer + Display APM on for captures and
    // option checks without touching the player's profile.
    let mut hud_stats = false;
    // Capture/acceptance overrides for the remaining Video/Game options.
    let mut pillarbox = false;
    let mut sd_graphics = false;
    let mut no_color_cycling = false;
    let mut minimap_standard = false;
    let mut no_bonus_skins = false;
    let mut show_fps = false;
    let mut show_turn_rate = false;
    // `--console-skin default|silver|gold`: equip a Collections console skin
    // for captures and acceptance checks (the screen stores the same value).
    let mut console_skin: Option<u8> = None;
    // `--announcer default|um|jeon|kim|umjeonkim|heyjini`: equip a Collections
    // announcer set for captures and acceptance checks.
    let mut announcer_arg: Option<u8> = None;
    // `--carbot`: equip the Carbot unit collection for captures and
    // acceptance checks (the Collections screen stores the same setting).
    let mut carbot = false;
    // `--language <code>` / `GALAXYCRAFT_LOCALE`: CASC locale directory
    // (`enus`, `frfr`, `zhcn`, ...).  Without either, the last language
    // picked in Options is read from the user cache.
    let mut locale_arg: Option<String> = None;
    // `--gamepad-probe SECONDS`: print the mapped frames the real gilrs
    // backend produces, then exit.  Used by `scripts/virtual_gamepad_test.py`
    // and for checking a physical controller without starting a match.
    let mut gamepad_probe: Option<f32> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--chk" => {
                chk = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--maps" => {
                maps_dir = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--tiles" => {
                tiles_dir = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--assets" => {
                assets = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            "--install" => {
                install = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--cache" => {
                cache = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--debug-input" => {
                // On-screen input readout: lets a player confirm that the
                // pointer and buttons reach the client on their session.
                debug_input = true;
                i += 1;
            }
            "--freeze" => {
                // Freeze the simulation once the start screen is set up, so a
                // live capture is comparable with the headless render.
                freeze = true;
                i += 1;
            }
            "--start-screen" => {
                // Open the interactive client straight on a screen (episode,
                // options, in-game, ...) so a live capture can be taken without
                // injecting keystrokes.
                start_screen = Some(args[i + 1].clone());
                i += 2;
            }
            "--fullscreen" => {
                // The default; kept for scripts that pass it.
                fullscreen = true;
                i += 1;
            }
            "--windowed" => {
                fullscreen = false;
                i += 1;
            }
            "--display" => {
                display_backend = args.get(i + 1).cloned();
                i += 2;
            }
            "--races" => {
                races = Some(args[i + 1].split(',').map(parse_race).collect());
                i += 2;
            }
            "--width" => {
                width = args[i + 1].parse().unwrap_or(width);
                i += 2;
            }
            "--height" => {
                height = args[i + 1].parse().unwrap_or(height);
                i += 2;
            }
            "--shot" => {
                shot = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--shot-screen" => {
                shot_screen = args[i + 1].clone();
                i += 2;
            }
            "--menu-script" => {
                menu_script = Some(args[i + 1].clone());
                i += 2;
            }
            "--replay" => {
                replay_path = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--import-characters" => {
                import_characters = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--game-script" => {
                game_script = Some(args[i + 1].clone());
                i += 2;
            }
            "--shot-map" => {
                shot_map = args[i + 1].parse().ok();
                i += 2;
            }
            "--menu-at" => {
                menu_at = args[i + 1].parse().ok();
                i += 2;
            }
            "--menu-phase" => {
                for part in args[i + 1].split(',') {
                    if let Some((name, value)) = part.split_once('=')
                        && let Ok(value) = value.parse()
                    {
                        menu_phases.insert(name.to_string(), value);
                    }
                }
                i += 2;
            }

            "--language" | "--locale" => {
                locale_arg = Some(args[i + 1].clone());
                i += 2;
            }
            "--gamepad-probe" => {
                gamepad_probe = args[i + 1].parse().ok();
                i += 2;
            }
            "--no-fog" => {
                no_fog = true;
                i += 1;
            }
            "--hud-stats" => {
                hud_stats = true;
                i += 1;
            }
            "--pillarbox" => {
                pillarbox = true;
                i += 1;
            }
            "--sd" => {
                sd_graphics = true;
                i += 1;
            }
            "--no-color-cycling" => {
                no_color_cycling = true;
                i += 1;
            }
            "--minimap-standard" => {
                minimap_standard = true;
                i += 1;
            }
            "--no-bonus-skins" => {
                no_bonus_skins = true;
                i += 1;
            }
            "--show-fps" => {
                show_fps = true;
                i += 1;
            }
            "--show-turn-rate" => {
                show_turn_rate = true;
                i += 1;
            }
            "--console-skin" => {
                console_skin = Some(match args[i + 1].as_str() {
                    "silver" => 1,
                    "gold" => 2,
                    _ => 0,
                });
                i += 2;
            }
            "--announcer" => {
                announcer_arg = Some(match args[i + 1].as_str() {
                    "um" => 1,
                    "jeon" => 2,
                    "kim" => 3,
                    "umjeonkim" => 4,
                    "heyjini" => 5,
                    _ => 0,
                });
                i += 2;
            }
            "--carbot" => {
                carbot = true;
                i += 1;
            }
            "--no-intro" => {
                no_intro = true;
                i += 1;
            }
            other => {
                eprintln!("unknown argument {other}");
                std::process::exit(2);
            }
        }
    }

    if let Some(seconds) = gamepad_probe {
        // Exercise the same backend the game loop uses.  The probe does not
        // need a window or an install, so it also serves as a quick
        // "is the controller seen?" check on a new machine.
        let mut pad = gamepad::Gamepad::new();
        println!("gamepad: {}", pad.describe());
        let started = Instant::now();
        let mut active = 0u32;
        let mut presses = 0u32;
        let mut cursor = (0.0f32, 0.0f32);
        while started.elapsed().as_secs_f32() < seconds {
            let frame = pad.poll();
            if frame.active {
                active += 1;
                cursor.0 += frame.cursor.0;
                cursor.1 += frame.cursor.1;
                for key in &frame.pressed {
                    presses += 1;
                    println!("gamepad: pressed {key:?}");
                }
                for key in &frame.menu_pressed {
                    if !frame.pressed.contains(key) {
                        println!("gamepad: menu {key:?}");
                    }
                }
                if frame.lmb {
                    println!("gamepad: south held (left click)");
                }
                if frame.primary {
                    println!("gamepad: primary (A)");
                }
                if frame.smart_order {
                    println!("gamepad: smart order (X)");
                }
                if frame.attack_move {
                    println!("gamepad: attack-move (Y)");
                }
                if frame.page_prev || frame.page_next {
                    println!("gamepad: action radials (LB/RB)");
                }
                if frame.shortcut_radial {
                    println!("gamepad: shortcut radial (RT)");
                }
                if frame.shift {
                    println!("gamepad: group radial / add (LT)");
                }
                if frame.map {
                    println!("gamepad: minimap pointer (View)");
                }
                if frame.nav_pressed != (false, false, false, false) {
                    println!("gamepad: d-pad {:?}", frame.nav_pressed);
                }
                if frame.pan != (0, 0) {
                    println!("gamepad: pan {:?}", frame.pan);
                }
            }
            std::thread::sleep(Duration::from_millis(16));
        }
        println!(
            "gamepad: {active} active frames, {presses} edge presses, cursor moved ({:.0}, {:.0})",
            cursor.0, cursor.1
        );
        std::process::exit(if active > 0 { 0 } else { 3 });
    }
    let locale_code = galaxycraft::retail::normalize_locale(
        &locale_arg
            .or_else(|| std::env::var("GALAXYCRAFT_LOCALE").ok())
            .or_else(|| std::fs::read_to_string(saved_locale_path()).ok())
            .unwrap_or_else(|| "enus".into()),
    );
    let mut retail_data: Option<Arc<GameData>> = None;
    let mut install_handle: Option<RetailInstall> = None;
    let mut ui_strings: std::collections::HashMap<String, String> = Default::default();
    let mut cef_strings: std::collections::HashMap<String, String> = Default::default();
    let mut campaign_maps: Vec<MapEntry> = Vec::new();
    // Without `--install`: `SC_INSTALL` & co., then the platform's default
    // folders (a packaged build is started without arguments).
    let install = install.or_else(|| RetailInstall::from_env().map(|i| i.root().to_path_buf()));
    if let Some(path) = install {
        let mut install = RetailInstall::open(&path).unwrap_or_else(|e| {
            eprintln!("cannot open retail install {}: {e}", path.display());
            std::process::exit(2);
        });
        install.set_locale(&locale_code);
        let cache = cache.unwrap_or_else(|| install.default_cache_dir());
        if let Err(e) = install.ensure_cache(&cache) {
            eprintln!(
                "cannot materialise retail assets into {}: {e}",
                cache.display()
            );
            std::process::exit(2);
        }
        eprintln!("retail assets: {}", cache.display());
        assets = cache.clone();
        maps_dir = cache.join("maps");
        tiles_dir = cache.join("TileSet");
        campaign_maps = scan_campaign(cache.as_path());
        eprintln!(
            "campaign: {} missions, {} melee maps",
            campaign_maps.len(),
            scan_maps(&maps_dir).len()
        );
        retail_data = install.game_data().ok().map(Arc::new);
        // Command cards come from the install's executable, so a build
        // without the developer fixtures (a release package) has them too.
        match install.button_sets() {
            Some(sets) => galaxycraft::button_sets::use_install_table(sets),
            None => eprintln!(
                "command cards: x86/StarCraft.exe table not found; using the embedded one"
            ),
        }
        // Hotkeys panel data, rebuilt from the install's CEF bundle and
        // stat_txt (same reason).
        let hotkeys = install
            .read("webui/dist/hotkeypanel/hotkeypanel.js")
            .and_then(|b| String::from_utf8(b).ok())
            .zip(retail_data.as_ref().map(|d| d.strings.clone()).or_else(|| {
                install
                    .read("rez/stat_txt.tbl")
                    .and_then(|b| galaxycraft::tables::StringTable::parse(&b).ok())
            }));
        if !hotkeys.is_some_and(|(js, tbl)| hotkey_data::HotkeyData::use_install(&js, &tbl)) {
            eprintln!("hotkeys: install panel data not found; using the embedded one");
        }
        install_handle = Some(install);
        ui_strings = load_ui_strings(install_handle.as_ref());
        cef_strings = load_cef_strings(install_handle.as_ref());
    }
    let Some(data) = retail_data.or_else(|| GameData::bundled().ok().map(Arc::new)) else {
        // A build without the developer fixtures needs the player's game.
        let msg = [
            "StarCraft: Remastered was not found.",
            "",
            "Start GalaxyCraft with --install <StarCraft folder>",
            "or set SC_INSTALL to that folder.",
        ];
        for line in msg {
            eprintln!("{line}");
        }
        if shot.is_none() {
            show_message_window(&msg);
        }
        std::process::exit(2);
    };
    // Campaign missions come from the same materialised cache; they are
    // listed first and keep their retail episode order.
    let mut maps = campaign_maps;
    let campaign_len = maps.len();
    maps.extend(scan_maps(&maps_dir));
    if maps.is_empty() {
        maps.push(MapEntry::new(
            asset_root().join("bottleneck.chk"),
            "Bottleneck".into(),
            2,
        ));
    }
    let menu = chk.is_none();
    let start_path = chk.clone().unwrap_or_else(|| maps[0].path.clone());
    let mut map_index = maps.iter().position(|m| m.path == start_path).unwrap_or(0);
    let mut campaign_len = campaign_len;
    let raw = std::fs::read(&start_path).expect("scenario CHK");
    let scenario = Scenario::parse(&raw).expect("valid CHK");
    let tileset = Tileset::load(&tiles_dir, scenario.era, &scenario).unwrap_or_else(|| {
        eprintln!(
            "missing tileset for era {} ({}); expected {}/{}.cv5",
            scenario.era,
            era_name(scenario.era),
            tiles_dir.display(),
            era_name(scenario.era)
        );
        std::process::exit(2);
    });
    let vf4 = std::fs::read(tiles_dir.join(format!("{}.vf4", era_name(scenario.era))))
        .expect("tileset vf4");
    // Player slots come from OWNR/SIDE, not from the number of start
    // locations: campaign missions place CPU bases in non-contiguous slots
    // (Enslavers uses 0/3/4/6), and the start-location count would drop them.
    let slots = scenario.player_slots();
    // `--chk` may name a file outside the bundled pack; give it a real menu
    // entry so the HUD, result panel and restart use its name.
    if let Some(path) = &chk
        && !maps.iter().any(|m| m.path == *path)
    {
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("map")
            .to_string();
        maps.insert(
            0,
            MapEntry::new(
                path.clone(),
                name,
                slots.iter().filter(|s| s.active).count().clamp(2, 8),
            ),
        );
        campaign_len += 1;
        map_index = 0;
    }
    let race_override = races.clone();
    let menu_race = race_override
        .as_ref()
        .and_then(|r| r.first().copied())
        .unwrap_or(Race::Terran);
    let races = races_for(&scenario, &slots, &race_override, menu_race);
    let mut world = World::from_scenario(
        data.clone(),
        &scenario,
        &galaxycraft::map::TileData::new(tileset.cv5(), &vf4),
        &races,
    );
    apply_slots(&mut world, &slots);
    world.enable_fog();

    let sprites = SpriteBank::load(&assets);
    let sounds = SoundBank::load(&assets.join("sound"));
    let audio = AudioPlayer::new(std::env::var("SC_NO_AUDIO").is_err());
    let local_player = slots
        .iter()
        .find(|s| s.active && s.human)
        .map(|s| s.slot)
        .unwrap_or(0);
    let console_race = slots
        .iter()
        .find(|s| s.slot == local_player)
        .map(|s| s.race(Race::Terran))
        .unwrap_or(Race::Terran);
    // Replaced after the profile is read, so the saved console skin applies.
    let console = ConsoleArt::load(&assets, console_race, 0, 0);
    let glue = {
        let art = GlueArt::load(&assets);
        (art.has_art() || art.menu.is_some() || art.score.is_some()).then_some(art)
    };
    let sfx_table = std::fs::read(assets.join("tables/sfxdata.tbl"))
        .ok()
        .and_then(|b| SfxTable::parse(&b))
        .unwrap_or_else(SfxTable::bundled);
    let voices = VoiceBank::new(assets.join("voices"), sfx_table);
    let music = MusicBank::new(assets.join("music"));
    let video_store = VideoStore::load(&assets);
    let (intro_info, intro_poster) = video_store
        .as_ref()
        .and_then(|v| v.intro_poster(false))
        .map(|(info, poster)| (Some(info), Some(poster)))
        .unwrap_or((None, None));
    // Retail main-menu animations: the HD clips carry the fleet (Single),
    // the battlecruiser (Multiplayer) and the vortex (Exit), with the hover
    // variants adding the tactical overlay/portals.  They are streamed with
    // their alpha at the rectangles from `rez/flc.json`.
    // `glue/mainmenu` carries the high-resolution clip the retail client
    // draws at 1440p (e.g. `multion.webm` is 1200x880 and includes the
    // targeting grid and corner brackets seen in the references);
    // `videos/hd2/mainmenu` is the smaller fallback.
    let clip_path = |name: &str| -> PathBuf {
        let hd = assets.join(format!("glue/mainmenu/{name}.webm"));
        if hd.is_file() {
            hd
        } else {
            assets.join(format!("videos/hd2/mainmenu/{name}.webm"))
        }
    };
    let clip_start = menu_at.unwrap_or(MENU_CLIP_START);
    let frozen = menu_at.is_some() || !menu_phases.is_empty();
    let load_clip = |name: &str| {
        let start = menu_phases.get(name).copied().unwrap_or(clip_start);
        MenuClip::open_at(clip_path(name), start, frozen)
    };
    let menu_layers = {
        let layers = MenuLayers {
            single: load_clip("single"),
            single_on: load_clip("singleon"),
            multi: load_clip("multi"),
            multi_on: load_clip("multion"),
            exit: load_clip("exit"),
            exit_on: load_clip("exiton"),
        };
        if layers.single.is_none() {
            eprintln!("menu: space layers unavailable (install without glue/mainmenu webm)");
            None
        } else {
            Some(layers)
        }
    };
    let intro = menu && !no_intro && glue.is_some();
    galaxycraft::team_colors::load(install_handle.as_ref());
    // Retail pointer skins, HD and SD (see `CursorShape`).
    let cursor_art = install_handle.as_ref().map(|inst| {
        let mut art = CursorArt::default();
        for shape in CursorShape::ALL {
            let stem = shape.file();
            if let Some(g) = inst
                .read_by_path(&format!("cursor/{stem}.dds.grp"))
                .ok()
                .and_then(|bytes| galaxycraft::dds::DdsGrp::parse(&bytes))
            {
                art.hd.insert(shape, g);
            }
            if let Some(g) = inst
                .read_by_path(&format!("cursor/{stem}.grp"))
                .ok()
                .and_then(|bytes| galaxycraft::render::Grp::parse(&bytes))
            {
                art.sd.insert(shape, g);
            }
        }
        art
    });
    let mut hotkey_bindings = hotkey_data::HotkeyData::get().default_bindings();
    if let Some(path) = hotkey_save_path()
        && let Some(saved) = hotkey_data::load_bindings(&path)
    {
        for (i, binding) in saved.iter().enumerate() {
            if i < hotkey_bindings.len() {
                hotkey_bindings[i] = *binding;
            }
        }
    }
    let hotkey_baseline = hotkey_bindings.clone();
    // Interactive sessions persist; a menu script only into an explicit
    // `GALAXYCRAFT_SAVES` (the walkthrough's temporary directory); shots and
    // `--start-screen` fixtures never touch the player's files.
    let persist_profiles = shot.is_none()
        && start_screen.is_none()
        && (menu_script.is_none() || std::env::var_os("GALAXYCRAFT_SAVES").is_some());
    let mut profile_store = load_profile_store(persist_profiles);
    if let Some(dir) = &import_characters {
        let added = profile_store.import_retail_dir(dir);
        eprintln!(
            "imported {added} retail Registry ID(s) from {}",
            dir.display()
        );
    }
    let profile_saved = profile_store.to_bytes();
    let mut app = App {
        data,
        scenario,
        world,
        tileset,
        sprites,
        sounds,
        audio,
        audio_background: true,
        assets_root: assets.clone(),
        console,
        glue,
        ui_font: galaxycraft::text::UiFont::load_retail(&assets),
        glue_font: galaxycraft::text::UiFont::load(&assets.join("font/eurostile-reg.ttf")),
        glue_font_ext: galaxycraft::text::UiFont::load(&assets.join("font/eurostileext-reg.ttf")),
        glue_font_small: galaxycraft::text::UiFont::load(&assets.join("font/bl.ttf")),
        hud_font: galaxycraft::text::UiFont::load(&assets.join(HUD_FONT)),
        ui_font_hover: galaxycraft::text::UiFont::load_retail_extended(&assets),
        ui_font_info: galaxycraft::text::UiFont::load(&assets.join(HUD_FONT)),
        music,
        voices,
        music_track: String::new(),
        music_started: Instant::now(),
        music_phase: 0,
        console_race,
        intro,
        intro_started: Instant::now(),
        menu_static: None,
        menu_static_bare: None,
        minimap_cache: None,
        plates_layer: None,
        present_offset: (0, 0),
        virtual_pointer: None,
        last_raw_pointer: None,
        intro_fit: None,
        backdrop_fit: None,
        moon_fit: None,
        options_plate: None,
        collections_layer: None,
        episode_static: None,
        episode_bare: None,
        video_store,
        intro_poster,
        intro_info,
        intro_expansion: false,
        intro_clips: Vec::new(),
        intro_subs: None,
        intro_caption_done: false,
        intro_clip: 0,
        intro_pending_map: None,
        intro_player: None,
        intro_probed: false,
        intro_last: Instant::now(),
        intro_audio: None,
        briefing_poster: None,
        briefing_video: None,
        briefing_player: None,
        briefing_player_key: None,
        briefing_last: Instant::now(),
        menu_screen: MenuScreen::Main,
        game_options: profile_store.settings.clone(),
        profile_store,
        profile_saved,
        persist_profiles,
        profile_sel: 0,
        registry_entry: None,
        registry_notice: None,
        main_cursor: None,
        gametype_hover: None,
        gametype_origin: galaxycraft::menu::GameTypeOrigin::SinglePlayer,
        registry_after_gametype: false,
        episode_sel: 0,
        mission_list: galaxycraft::campaign_progress::MissionList::default(),
        mission_drag: None,
        campaign_prompt: None,
        lobby_races: vec![LobbyRace::Random; 8],
        game_dialog: None,
        game_dialog_hover: None,
        glue_disabled: Vec::new(),
        save_name: String::new(),
        dialog_list_sel: None,
        help_lines: std::fs::read_to_string(assets.join("ui/help_txt.json"))
            .map(|t| galaxycraft::menu::parse_help_lines(&t))
            .unwrap_or_default(),
        help_scroll: 0,
        ingame_options: false,
        map_locations: [None; 3],
        order_flash: None,
        order_marker: None,
        card_pressed: None,
        tips_text: std::fs::read_to_string(assets.join("ui/tips_txt.json"))
            .map(|t| galaxycraft::menu::parse_tips(&t))
            .unwrap_or_default(),
        tip_index: 0,
        startup_tips: shot.is_none()
            && start_screen.is_none()
            && menu_script.is_none()
            && game_script.is_none(),
        credits_view: CreditsView::default(),
        show_stats: false,
        show_leaderboard: false,
        minimap_terrain: true,
        content_box: (0, 0, width as i32, height as i32),
        ui_scale: 2.0,
        shot_name: shot_screen.clone(),
        ui_ox: 0,
        ui_oy: 0,
        result_panel: None,
        score_tab: 0,
        score_epoch: None,
        score_leave: None,
        score_count_epoch: None,
        score_art: None,
        score_icons: None,
        nation_names: None,
        result_fit: None,
        ack_variant: 0,
        select_streak: None,
        unit_types_seen: std::collections::HashMap::new(),
        actions: 0,
        fps: 0.0,
        selected: Vec::new(),
        groups: Default::default(),
        ais: Vec::new(),
        campaign_ais: Vec::new(),
        maps,
        local_player,
        campaign_len,
        menu_tab: if map_index < campaign_len { 0 } else { 1 },
        menu_index: if map_index < campaign_len {
            map_index
        } else {
            map_index - campaign_len
        },
        map_index,
        menu,
        briefing: false,
        briefing_pages: Vec::new(),
        briefing_page: 0,
        briefing_page_epoch: Instant::now(),
        briefing_due: None,
        ready_script: None,
        ready: None,
        ready_epoch: Instant::now(),
        ready_race: galaxycraft::sim::Race::Terran,
        ready_hover: None,
        ready_at: None,
        ready_players: std::array::from_fn(|i| {
            galaxycraft::portrait::PortraitPlayer::new(i as u32 + 1)
        }),
        ready_glow: None,
        ready_proj: None,
        ready_proloop: None,
        briefing_art: None,
        briefing_fit: None,
        paused: false,
        game_result: None,
        end_dialog: false,
        end_dialog_hold: 0,
        result_tick: 0,
        game_error: None,
        next_map: None,
        tiles_dir: tiles_dir.clone(),
        menu_race,
        hd_terrain: None,
        hd_sprites: Default::default(),
        hd_lights: install_handle
            .as_ref()
            .and_then(|inst| inst.read("anim/main.lit"))
            .and_then(|b| galaxycraft::hd_lights::LitTable::parse(&b)),
        melee_slot: None,
        local_team_color: None,
        creep_variants: Default::default(),
        creep_tiles: Default::default(),
        crt_rand: galaxycraft::render::CrtRand::new(crt_session_seed(shot.is_some())),
        headless: shot.is_some(),
        portrait_data: None,
        portrait_player: galaxycraft::portrait::PortraitPlayer::new(crt_session_seed(
            shot.is_some(),
        )),
        portrait_clips: Default::default(),
        portrait_talk: None,
        portrait_talk_started: false,
        trigger_portraits_seen: (0, 0),
        trigger_cues_seen: 0,
        center_views_seen: 0,
        rescues_seen: 0,
        trigger_text: Default::default(),
        trigger_text_epoch: Instant::now(),
        portrait_epoch: Instant::now(),
        portrait_pick: None,
        unit_poses: Default::default(),
        smooth_facing: Default::default(),
        smooth_facing_tick: 0,
        env_particle: None,
        env_particle_loaded: false,
        geyser_smoke: None,
        races: race_override,
        camera: (0, 0),
        view: (width as i32, height as i32 - HUD_H),
        drag_start: None,
        drag_now: (0, 0),
        attack_mode: false,
        patrol_mode: false,
        build_mode: None,
        build_list: Vec::new(),
        command_list: Vec::new(),
        card_set: None,
        target_act: None,
        rmb_minimap: false,
        card_owner: None,
        status: String::new(),
        status_until: Instant::now(),
        chat_log: VecDeque::new(),
        chat_input: String::new(),
        chat_active: false,
        speed: 1.0,
        fog: true,
        last_step: Instant::now(),
        accumulator: 0.0,
        rmb_down: false,
        lmb_down: false,
        quit: false,
        chk_raw: Vec::new(),
        last_races: Vec::new(),
        save_sel: 0,
        saves_dir: save_dirs().0,
        replays_dir: save_dirs().1,
        replay: None,
        net: None,
        lan: LanUi::default(),
        menu_layers,
        glue_swish: None,
        swish_restart: false,
        swish_screen: None,
        swish_drawn: std::cell::Cell::new(None),
        campaign_layers: None,
        episode_hover: None,
        menu_hover: None,
        lobby_race_open: None,
        sound_log: Vec::new(),
        collection_sel: 0,
        collections_scroll: 0.0,
        lobby_dir: Vec::new(),
        lobby_top: 0,
        lobby_game_type: galaxycraft::multiplayer::GameType::Melee,
        menu_at,
        menu_phases,
        lobby_preview: None,
        mp: mp::MpState::default(),
        hotkey_hover: None,
        hotkey_sel: None,
        hotkey_cells: Vec::new(),
        hotkey_action_cells: Vec::new(),
        hotkey_action: None,
        hotkey_bindings,
        hotkey_baseline,
        hotkey_unbound: Vec::new(),
        hotkey_binding_error: false,
        hotkey_tab_hover: None,
        capture_request: false,
        hotkey_reset: false,
        options_reset: false,
        options_drop: None,
        apm_input: None,
        apm_alerted: false,
        display_mode: 1,
        language: language_index(&locale_code),
        locale_code,
        ui_strings,
        cef_strings,
        changing_language: false,
        window_fullscreen: None,
        language_font: None,
        language_font_loaded: false,
        animator: galaxycraft::unit_anim::Animator::default(),
        image_names: None,
        sd_image_grps: std::collections::HashMap::new(),
        seen_buildings: std::collections::HashSet::new(),
        edge_scroll: None,
        deferred_cheats: Vec::new(),
        remap_tables: std::collections::HashMap::new(),
        volumes_sent: None,
        content_x0: 0,
        mouse_was_down: false,
        debug_input,
        last_key: None,
        pointer: None,
        gamepad: gamepad::Gamepad::new(),
        pad_frame: gamepad::PadFrame::default(),
        pad_cursor: (640, 360),
        pad_control: false,
        pad_suppressed: false,
        pad_drag: None,
        last_alert_pos: None,
        last_health: Vec::new(),
        pad_wheel: gamepad::WheelState::default(),
        pad_wheel_pages: Vec::new(),
        pad_tap: gamepad::TapTracker::default(),
        pad_started: Instant::now(),
        pad_last_poll: Instant::now(),
        pad_wheel_kind: pad_ui::RadialKind::default(),
        pad_lt_tap: gamepad::TriggerTap::default(),
        pad_minimap_mode: false,
        pad_group_at: 0,
        pad_hint_until: Instant::now(),
        boxed_fb: None,
        overlay: 0,
        install: install_handle,
        cursor_art: cursor_art.clone(),
        cursor_started: Instant::now(),
        portraits: std::collections::HashMap::new(),
        credits: Vec::new(),
        thumbs: std::collections::HashMap::new(),
        hotkey_icons: std::collections::HashMap::new(),
    };
    if app.intro {
        // First launch: the publisher logo followed by the title movie, both
        // read from the retail intro table.
        app.begin_intro(
            galaxycraft::video::intro_sequence(false, true)
                .into_iter()
                .map(str::to_string)
                .collect(),
            None,
        );
    }
    if no_fog {
        app.fog = false;
    }
    if hud_stats {
        app.game_options.game_timer = true;
        app.game_options.apm_display = true;
    }
    if pillarbox {
        app.game_options.video_pillarbox = true;
    }
    if sd_graphics {
        app.game_options.video_hd = false;
    }
    if no_color_cycling {
        app.game_options.game_color_cycling = false;
    }
    if minimap_standard {
        app.game_options.minimap_position = 1;
    }
    if no_bonus_skins {
        app.game_options.game_bonus_skins = false;
    }
    if show_fps {
        app.game_options.show_fps = true;
    }
    if show_turn_rate {
        app.game_options.show_turn_rate = true;
    }
    if carbot {
        app.game_options.game_unit_skin = 2;
    }
    if let Some(skin) = console_skin {
        app.game_options.game_console_skin = skin;
    }
    if let Some(id) = announcer_arg {
        app.game_options.game_announcer = id;
    }
    app.voices.set_announcer(app.game_options.game_announcer);
    app.console = ConsoleArt::load(
        &assets,
        console_race,
        app.game_options.game_console_skin,
        app.game_options.game_unit_skin,
    );
    app.ais = slots
        .iter()
        .filter(|s| s.computer && skirmish_ai(&app.world, s.slot))
        .map(|s| SimpleAi::new(s.slot as u8))
        .collect();
    app.center_on_base();
    app.refresh_commands();
    if app.intro || app.menu {
        app.start_menu_music();
    } else {
        app.start_mission_music();
    }

    if let Some(path) = &replay_path {
        // Parse first so a bad file fails loudly instead of opening the menu.
        if galaxycraft::replay::SaveFile::load(path).is_none() {
            eprintln!(
                "cannot replay {}: not a GalaxyCraft save/replay",
                path.display()
            );
            std::process::exit(1);
        }
        app.load_save(path, true);
    }

    let supersample =
        shot.is_some() && std::env::var("GC_SUPERSAMPLE").ok().as_deref() == Some("1");
    let mut fb = if supersample {
        Framebuffer::new(3413, 1920)
    } else {
        // Narrower-than-16:9 windows draw into the letterboxed 16:9 surface
        // (`letterbox`); shots are composed back to the window size.
        let (iw, ih, ox, oy) = letterbox(width, height);
        app.present_offset = (ox, oy);
        Framebuffer::new(iw, ih)
    };
    // The view (and so every start camera `load_map` computes) must follow
    // this surface from the start: it used to stay at the default window's
    // until the first draw, so captures at other sizes framed the map from
    // a different camera.
    app.update_view(&fb);
    // Headless screen capture for UI verification: the exact same draw code
    // runs, but into a BMP instead of a window.  This lets the campaign
    // menu, retail briefing and mission screens be checked without an X
    // server or a window manager.
    if let Some(script) = game_script {
        if let Some(index) = shot_map {
            app.map_index = index.min(app.maps.len().saturating_sub(1));
        }
        run_game_script(&mut app, &mut fb, &script);
        return;
    }
    if let Some(script) = menu_script {
        app.intro = false;
        app.menu = true;
        app.menu_screen = MenuScreen::Main;
        app.profile_sel = 0;
        run_menu_script(&mut app, &mut fb, &script);
        return;
    }
    if shot.is_some() || start_screen.is_some() {
        // The reference session's profile is `LinKa`; captures use it so the
        // Registry/Create/Lobby leader lines match the pictures, while live
        // play keeps the player's own active profile.
        if let Some(i) = app
            .profile_store
            .profiles
            .iter()
            .position(|p| p.name == "LinKa")
        {
            app.profile_store.active = Some(i);
        }
        if let Some(name) = &start_screen {
            shot_screen = name.clone();
        }
        if let Some(index) = shot_map {
            app.map_index = index.min(app.maps.len().saturating_sub(1));
        }
        match shot_screen.as_str() {
            "intro" | "intro-blizzard" | "intro-title" | "intro-expansion" => {
                // `intro-blizzard` captures the separate publisher logo,
                // `intro`/`intro-title` the title movie at 5 s/25 s and
                // `intro-expansion` the Brood War intro.
                let (clip, at) = match shot_screen.as_str() {
                    "intro-blizzard" => (galaxycraft::video::BLIZZARD_LOGO, 5),
                    "intro-expansion" => (galaxycraft::video::INTRO_EXPANSION, 25),
                    "intro-title" => (galaxycraft::video::INTRO_ORIGINAL, 25),
                    _ => (galaxycraft::video::INTRO_ORIGINAL, 5),
                };
                app.begin_intro(vec![clip.to_string()], None);
                app.intro_expansion = clip == galaxycraft::video::INTRO_EXPANSION;
                // The title movies carry no typed establishing caption, and
                // its first-frame path resets the clock; skip it so `at`
                // reaches the decoder (the 5 s `Blizzard Entertainment
                // Presents` card used to capture as a black frame).
                app.intro_caption_done = true;
                app.intro_started = Instant::now() - Duration::from_secs(at);
                app.draw_intro(&mut fb);
            }
            // Frame of a replay opened with `--replay` (the world is already
            // built and fast-forwarded; draw it as-is).
            "replay" => {
                app.update_view(&fb);
                app.centre_on_local_unit();
                app.world.update_vision();
                app.update();
                app.draw(&mut fb);
            }
            // Plain in-game frame with units selected: no dialog overlays.
            // Trigger transmission end to end: 1. Wasteland runs until its
            // first trigger text and sounds, through the client's own cue
            // handling (text above the console, map WAV resolved + decoded).
            "ingame-transmission" => {
                if let Some(index) = app.campaign_map_index(2) {
                    app.map_index = index;
                }
                app.load_map(app.map_index);
                app.prepare_ingame_shot(&fb);
                let (mut lines, mut sounds) = (0, Vec::new());
                for _ in 0..24 * 90 {
                    app.world.step();
                    let (l, s) = app.follow_trigger_cues();
                    lines += l;
                    sounds.extend(s);
                    if lines > 0 && !sounds.is_empty() {
                        break;
                    }
                }
                eprintln!(
                    "transmission: tick {} map {} text lines {lines} sounds {sounds:?}",
                    app.world.tick,
                    app.maps
                        .get(app.map_index)
                        .map(|m| m.name.as_str())
                        .unwrap_or("?")
                );
                app.draw(&mut fb);
            }
            "ingame" => {
                app.load_map(app.map_index);
                app.prepare_ingame_shot(&fb);
                app.update();
                app.draw(&mut fb);
            }
            // Race-specific in-game captures: load that race's campaign
            // mission so the HUD console matches the reference screens.
            // The user's (8)The Hunters games of 2026-10-27: Zerg as slot 1
            // (134905), Terran as slot 7 (135020), Protoss as slot 6
            // (135051..135121); start locations identified from the
            // geyser/mineral layout around each town hall.
            "ingame-zerg"
            | "ingame-terran2"
            | "ingame-protoss"
            | "ingame-protoss-tip-shields"
            | "ingame-protoss-tip-probe"
            | "ingame-protoss-tip-minimap" => {
                let (slot, race, dy) = match shot_screen.as_str() {
                    // Zerg nudge: mineral-mask alignment on 134905.
                    "ingame-zerg" => (1, Race::Zerg, (-6, 8)),
                    "ingame-terran2" => (7, Race::Terran, (0, -8)),
                    _ => (6, Race::Protoss, (0, 8)),
                };
                app.hunters_start(&fb, slot, race, dy);
                if let Some(smoke) = hunters_capture_smoke(shot_screen.as_str()) {
                    app.set_geyser_smoke(smoke);
                }
                app.minimap_terrain = hunters_capture_minimap_terrain(shot_screen.as_str());
                // Advisor idle clip and frame each capture shows (matched
                // over every frame of padfid00..03; 10 fps, mid-frame time).
                app.portrait_pick = match shot_screen.as_str() {
                    "ingame-protoss" => Some((0, 0.95)),
                    "ingame-protoss-tip-shields" => Some((3, 0.35)),
                    "ingame-protoss-tip-probe" | "ingame-protoss-tip-minimap" => Some((1, 0.75)),
                    _ => None,
                };
                app.update();
                // 135109 / 135114 / 135121 hover the shield upgrade, the
                // Probe button and the minimap terrain toggle.
                let centre = |r: UiRect| (r.x + r.w / 2, r.y + r.h / 2);
                app.pointer = match shot_screen.as_str() {
                    "ingame-protoss-tip-shields" => Some(centre(app.upgrade_button_rect())),
                    "ingame-protoss-tip-probe" => Some(centre(app.command_slots()[0])),
                    "ingame-protoss-tip-minimap" => Some(centre(app.hud_control(
                        "minimap",
                        2,
                        UiRect::new(3, 320, 64, 24),
                    ))),
                    _ => None,
                };
                app.draw(&mut fb);
            }
            "ingame-terran" => {
                let race = shot_screen.trim_start_matches("ingame-").to_string();
                // Map choice is calibrated against the reference frames by a
                // measured sweep over the installed maps (see session log):
                // the Terran references are melee games, the Zerg and Protoss
                // ones line up best with these campaign missions.
                let mission = match race.as_str() {
                    "zerg" => "06.chk",
                    // Ref 41-44 are one Protoss game; protoss09 is the mission
                    // whose terrain lines up best (measured sweep).
                    "protoss" => "09.chk",
                    _ => "02.chk",
                };
                let want = format!("/{}/", race.trim_end_matches('2'));
                if let Some(i) = app.maps.iter().position(|m| {
                    let p = m.path.to_string_lossy();
                    p.contains(&want) && p.ends_with(mission)
                }) {
                    app.map_index = i;
                }
                // A melee map (measured best fit) overrides the campaign pick.
                let melee = match race.as_str() {
                    "terran" => Some("volcanis"),
                    "terran2" => Some("space madness"),
                    _ => None,
                };
                if let Some(name) = melee
                    && let Some(i) = app
                        .maps
                        .iter()
                        .position(|m| m.name.to_lowercase().contains(name))
                {
                    app.map_index = i;
                }
                // Capture calibration: a shot may be pointed at any installed
                // map by name, which is how the reference map is identified by
                // measurement instead of by guesswork.
                if let Ok(want) = std::env::var("GC_SHOT_MAP") {
                    let want = want.to_lowercase();
                    if let Some(i) = app
                        .maps
                        .iter()
                        .position(|m| m.name.to_lowercase().contains(&want))
                    {
                        app.map_index = i;
                    }
                }
                app.load_map(app.map_index);
                app.prepare_ingame_shot(&fb);
                app.update();
                app.draw(&mut fb);
            }
            "menu" | "main" | "menu-hover-single" | "menu-hover-multi" | "menu-hover-exit" => {
                app.intro = false;
                app.menu = true;
                app.menu_screen = MenuScreen::Main;
                app.main_cursor = match shot_screen.as_str() {
                    "menu-hover-single" => Some(0),
                    "menu-hover-multi" => Some(1),
                    "menu-hover-exit" => Some(7),
                    _ => None,
                };
                app.draw_menu(&mut fb);
            }
            "registry" | "registry-new-id" | "registry-new-id-typed" | "registry-name-taken" => {
                app.intro = false;
                app.menu = true;
                app.menu_screen = MenuScreen::Registry;
                if shot_screen == "registry-name-taken" {
                    app.registry_notice = galaxycraft::profile::NameError::Duplicate.notice();
                } else if shot_screen != "registry" {
                    let mut entry = galaxycraft::profile::NameEntry::default();
                    if shot_screen == "registry-new-id-typed" {
                        for c in "LinKa".chars() {
                            entry.push(c);
                        }
                    }
                    app.registry_entry = Some(entry);
                }
                app.draw_menu(&mut fb);
            }
            "gametype" | "gametype-hover-expansion" => {
                app.intro = false;
                app.menu = true;
                app.menu_screen = MenuScreen::GameType;
                app.gametype_hover = (shot_screen == "gametype-hover-expansion").then_some(1);
                app.draw_menu(&mut fb);
            }
            "episode"
            | "episode-hover-zerg"
            | "episode-hover-terran"
            | "episode-hover-protoss"
            | "episode-expansion-hover-zerg"
            | "episode-expansion-hover-terran"
            | "episode-expansion-hover-protoss"
            | "episode-locked-zerg" => {
                let screen = shot_screen.as_str();
                app.intro = false;
                app.menu = true;
                app.menu_screen = MenuScreen::Episode;
                app.set_reference_progress();
                if screen == "episode-locked-zerg" {
                    // Live capture: the prompt after clicking the locked Zerg.
                    app.campaign_prompt = Some(galaxycraft::campaign_progress::Campaign::Zerg);
                }
                let expansion = screen.contains("expansion");
                app.episode_sel = galaxycraft::menu::episode_start(expansion);
                // Slot order differs per campaign set: original is
                // Zerg/Terran/Protoss, Brood War is Terran/Protoss/Zerg.
                app.episode_hover = if screen.ends_with("hover-zerg") {
                    Some(if expansion { 2 } else { 0 })
                } else if screen.ends_with("hover-terran") {
                    Some(if expansion { 0 } else { 1 })
                } else if screen.ends_with("hover-protoss") {
                    Some(if expansion { 1 } else { 2 })
                } else {
                    None
                };
                // The portraits fade in during the first second (reference
                // 130744); headless captures otherwise show only the rings.
                if app.menu_at.is_none() {
                    app.menu_at = Some(1.0);
                }
                app.draw_menu(&mut fb);
            }
            "mission" | "mission-live" | "mission-live-scrolled" => {
                app.intro = false;
                app.menu = true;
                app.set_reference_progress();
                let terran = galaxycraft::campaign_progress::Campaign::Terran;
                if shot_screen != "mission" {
                    // Live profile LinKa: history up to `9. New Gettysburg`.
                    let mut p = app.progress();
                    p.next[terran.index()] = 10;
                    app.set_progress(p);
                }
                app.enter_campaign(terran);
                // 131132 was wheel-scrolled to the last page with the
                // default selection (entry 0) out of view.
                match shot_screen.as_str() {
                    "mission" => app.mission_list.scroll(10),
                    "mission-live-scrolled" => {
                        app.mission_list.click_row(2);
                        app.mission_list.scroll(1);
                    }
                    _ => {}
                }
                app.episode_hover = None;
                app.draw_menu(&mut fb);
            }
            "lobby" | "create" | "lobby-chat" | "lobby-hunters" => {
                app.intro = false;
                app.menu = true;
                if app.campaign_len < app.maps.len() {
                    app.map_index = app.campaign_len;
                }
                // The 2026-11-07 `Play Custom` captures open on Bottleneck
                // with the map tree at its root (39-42); 43/44 pick The
                // Hunters and show the 8-player page.
                app.lobby_dir.clear();
                app.lobby_top = 0;
                let want = if shot_screen == "lobby-hunters" {
                    "The Hunters"
                } else {
                    "Bottleneck"
                };
                if let Some(i) = app
                    .maps
                    .iter()
                    .position(|m| m.campaign.is_none() && m.name.contains(want))
                {
                    app.map_index = i;
                }
                if shot_screen == "lobby-hunters" {
                    let rows = app.classic_rows();
                    if let Some(pos) = rows.iter().position(|r| {
                        matches!(r, ClassicRow::Map(i) if app.maps[*i].name.contains("Killing Fields"))
                    }) {
                        app.lobby_top = pos;
                    }
                }
                app.menu_screen = MenuScreen::Lobby {
                    map: Some(app.map_index),
                };
                app.draw_menu(&mut fb);
            }
            "options"
            | "options-speed"
            | "options-sound"
            | "options-video"
            | "options-game"
            | "options-network"
            | "options-reset"
            | "options-display-open"
            | "options-language-open"
            | "options-language-dialog"
            | "options-tooltip-scaling"
            | "options-tooltip-confine"
            | "options-tooltip-display"
            | "options-tooltip-sensitivity" => {
                let tab = match shot_screen.as_str() {
                    "options-sound" => OptionsTab::Sound,
                    "options-video" | "options-display-open" | "options-tooltip-display" => {
                        OptionsTab::Video
                    }
                    "options-game" | "options-language-open" | "options-language-dialog" => {
                        OptionsTab::Game
                    }
                    "options-network" | "options-reset" => OptionsTab::Network,
                    _ => OptionsTab::Speed,
                };
                app.intro = false;
                app.menu = true;
                app.options_reset = shot_screen == "options-reset";
                match shot_screen.as_str() {
                    "options-display-open" => app.options_drop = Some(OptionsDrop::DisplayMode),
                    "options-language-open" => app.options_drop = Some(OptionsDrop::Language),
                    "options-language-dialog" => {
                        app.language = 4;
                        app.changing_language = true;
                    }
                    "options-tooltip-scaling"
                    | "options-tooltip-confine"
                    | "options-tooltip-display"
                    | "options-tooltip-sensitivity" => {
                        app.update_view(&fb);
                        let (cx, _cy, cw, ch) = App::content_rect(&fb);
                        let at = |x: f32, y: f32| {
                            (
                                cx + (cw as f32 * x / 2560.0) as i32,
                                (ch as f32 * y / 1440.0) as i32,
                            )
                        };
                        app.pointer = Some(match shot_screen.as_str() {
                            "options-tooltip-scaling" => at(1468.0, 510.0),
                            "options-tooltip-confine" => at(1468.0, 618.0),
                            "options-tooltip-display" => at(1010.0, 352.0),
                            _ => at(1468.0, 352.0),
                        });
                    }
                    _ => {}
                }
                app.menu_screen = MenuScreen::Options { tab };
                app.draw_menu(&mut fb);
            }
            screen if screen.starts_with("hotkeys-any-") => {
                let rest = &screen["hotkeys-any-".len()..];
                let (race, id) = rest.split_once('-').unwrap_or(("terran", "7"));
                let race = match race {
                    "protoss" => HotkeyRace::Protoss,
                    "zerg" => HotkeyRace::Zerg,
                    _ => HotkeyRace::Terran,
                };
                let id = id.parse::<u16>().unwrap_or(7);
                app.intro = false;
                app.menu = true;
                app.hotkey_bindings = hotkey_data::HotkeyData::get().default_bindings();
                app.hotkey_unbound.clear();
                app.hotkey_binding_error = false;
                app.menu_screen = MenuScreen::Hotkeys { race };
                app.hotkey_sel = Some(id);
                app.hotkey_action = None;
                app.draw_menu(&mut fb);
            }
            "hotkeys"
            | "hotkeys-terran"
            | "hotkeys-protoss"
            | "hotkeys-zerg"
            | "hotkeys-terran-scv"
            | "hotkeys-protoss-probe"
            | "hotkeys-zerg-larva"
            | "hotkeys-zerg-larva-cards"
            | "hotkeys-terran-marine"
            | "hotkeys-terran-hover-scv"
            | "hotkeys-terran-scv-move"
            | "hotkeys-terran-scv-move-unbound"
            | "hotkeys-reset-dialog"
            | "hotkeys-terran-cc" => {
                let (race, sel, hover, action) = match shot_screen.as_str() {
                    "hotkeys-protoss" => (HotkeyRace::Protoss, None, None, None),
                    "hotkeys-zerg" => (HotkeyRace::Zerg, None, None, None),
                    "hotkeys-terran-scv" => (HotkeyRace::Terran, Some(7), None, None),
                    "hotkeys-protoss-probe" => (HotkeyRace::Protoss, Some(64), None, None),
                    "hotkeys-zerg-larva" => (HotkeyRace::Zerg, Some(35), None, Some(578)),
                    "hotkeys-zerg-larva-cards" => (HotkeyRace::Zerg, Some(35), None, None),
                    "hotkeys-terran-marine" => (HotkeyRace::Terran, Some(0), None, None),
                    "hotkeys-terran-hover-scv" => (HotkeyRace::Terran, None, None, None),
                    "hotkeys-terran-scv-move" => (HotkeyRace::Terran, Some(7), None, Some(664)),
                    "hotkeys-terran-scv-move-unbound" => {
                        (HotkeyRace::Terran, Some(7), None, Some(664))
                    }
                    "hotkeys-reset-dialog" => (HotkeyRace::Terran, Some(7), None, None),
                    "hotkeys-terran-cc" => (HotkeyRace::Terran, Some(106), None, None),
                    _ => (HotkeyRace::Terran, None, None, None),
                };
                app.intro = false;
                app.menu = true;
                app.hotkey_bindings = hotkey_data::HotkeyData::get().default_bindings();
                app.hotkey_unbound.clear();
                app.hotkey_binding_error = false;
                app.menu_screen = MenuScreen::Hotkeys { race };
                app.hotkey_sel = sel;
                app.hotkey_tab_hover = hover;
                app.hotkey_action = action;
                app.hotkey_hover = (shot_screen == "hotkeys-terran-hover-scv").then_some(7);
                if shot_screen == "hotkeys-terran-scv-move-unbound" {
                    app.rebind_hotkey(664, 'H', 'h' as u16);
                }
                app.hotkey_reset = shot_screen == "hotkeys-reset-dialog";
                app.draw_menu(&mut fb);
            }
            "collections" | "collections-announcers" | "collections-carbot" => {
                app.intro = false;
                app.menu = true;
                app.menu_screen = MenuScreen::Collections;
                app.collection_sel = if shot_screen == "collections-carbot" {
                    4
                } else {
                    0
                };
                if shot_screen == "collections-carbot" {
                    app.game_options.game_unit_skin = 2;
                }
                app.collections_scroll = if shot_screen == "collections-announcers" {
                    225.0
                } else {
                    0.0
                };
                app.draw_menu(&mut fb);
            }
            "credits" => {
                app.intro = false;
                app.menu = true;
                app.open_credits();
                app.draw_menu(&mut fb);
            }
            "saved" => {
                app.intro = false;
                app.menu = true;
                app.menu_screen = MenuScreen::SavedGames;
                app.draw_menu(&mut fb);
            }
            "lan" => {
                app.intro = false;
                app.menu = true;
                app.menu_screen = MenuScreen::Lan;
                app.lan.map_index = app.map_index;
                app.draw_menu(&mut fb);
            }
            "replays" => {
                app.intro = false;
                app.menu = true;
                app.menu_screen = MenuScreen::Replays;
                app.draw_menu(&mut fb);
            }
            "gamemenu"
            | "gamemenu-end"
            | "gamemenu-quit"
            | "gamemenu-exit"
            | "gamemenu-mp"
            | "gamemenu-mp-end"
            | "gamemenu-tips"
            | "gamemenu-save"
            | "gamemenu-load"
            | "gamemenu-help"
            | "gamemenu-keys"
            | "gamemenu-objectives"
            | "gamemenu-network" => {
                app.load_map(app.map_index);
                app.prepare_ingame_shot(&fb);
                // The mission's triggers set its objectives in the first
                // frames.
                for _ in 0..8 {
                    app.world.step();
                }
                let dialog = match shot_screen.as_str() {
                    "gamemenu-end" | "gamemenu-mp-end" => GameDialog::EndMission,
                    "gamemenu-quit" => GameDialog::ConfirmQuit,
                    "gamemenu-exit" => GameDialog::ConfirmExit,
                    "gamemenu-tips" => {
                        // 478 s: the first tip after the tutorial one.
                        app.tip_index = 1;
                        GameDialog::Tips
                    }
                    "gamemenu-save" => {
                        // Boot Camp 124 s: a name being typed.
                        app.save_name = "Este es un sav".to_string();
                        GameDialog::SaveGame
                    }
                    "gamemenu-load" => {
                        app.dialog_list_sel = (!app.save_list(false).is_empty()).then_some(0);
                        GameDialog::LoadGame
                    }
                    "gamemenu-help" => GameDialog::HelpMenu,
                    "gamemenu-keys" => GameDialog::KeystrokeHelp,
                    "gamemenu-objectives" => GameDialog::Objectives,
                    _ => GameDialog::Menu,
                };
                if shot_screen.contains("-mp") {
                    app.mp.setup = Some(galaxycraft::multiplayer::GameSetup {
                        game_type: galaxycraft::multiplayer::GameType::Melee,
                        slots: Vec::new(),
                    });
                }
                app.game_dialog = Some(dialog);
                if shot_screen == "gamemenu-network" {
                    // Boot Camp 151 s: Options over the game, Network tab.
                    app.open_ingame_options();
                    app.menu_screen = MenuScreen::Options {
                        tab: OptionsTab::Network,
                    };
                }
                app.draw(&mut fb);
            }
            "objectives" | "tips" => {
                // 134919 / 134931: the Zerg game of 134905 with the dialog open.
                app.hunters_start(&fb, 1, Race::Zerg, (-6, 8));
                if let Some(smoke) = hunters_capture_smoke(shot_screen.as_str()) {
                    app.set_geyser_smoke(smoke);
                }
                app.update();
                app.draw(&mut fb);
                if shot_screen == "objectives" {
                    app.draw_objectives_panel(&mut fb);
                } else {
                    app.draw_tips_panel(&mut fb);
                }
            }
            "ready" => {
                // Ready room at script time `--menu-at` seconds.
                app.load_map(app.map_index);
                app.briefing = true;
                app.ready_at = Some((app.menu_at.unwrap_or(0.0) * 1000.0) as u64);
                app.enter_ready_room();
                app.draw(&mut fb);
            }
            "briefing" => {
                app.load_map(app.map_index);
                app.briefing = app.briefing || !app.briefing_pages.is_empty();
                // `GC_BRIEF_PAGE=n`: capture page n of the establishing script.
                if let Some(n) = std::env::var("GC_BRIEF_PAGE")
                    .ok()
                    .and_then(|v| v.parse::<usize>().ok())
                {
                    app.briefing_page = n.min(app.briefing_pages.len().saturating_sub(1));
                    app.refresh_briefing_poster();
                }
                app.draw(&mut fb);
            }
            "pause" => {
                app.load_map(app.map_index);
                app.briefing = false;
                app.open_game_dialog(GameDialog::Menu);
                app.draw(&mut fb);
            }
            "select-cc" => {
                app.load_map(app.map_index);
                app.briefing = false;
                let me = app.local_player as u8;
                if let Some(id) = app
                    .world
                    .units
                    .iter()
                    .find(|u| {
                        u.alive
                            && u.owner == me
                            && u.type_id == galaxycraft::ids::unit::COMMAND_CENTER
                    })
                    .map(|u| u.id)
                {
                    app.selected = vec![id];
                    app.refresh_commands();
                }
                app.draw(&mut fb);
            }
            "leaderboard-marines" => {
                // Wasteland after its Barracks trigger: the Marine counter.
                app.load_map(app.map_index);
                app.briefing = false;
                let me = app.local_player;
                if let Some(t) = app.world.triggers.as_mut() {
                    t.leaderboard_computer_players = false;
                    t.leaderboard = Some(galaxycraft::trigger::Leaderboard {
                        player: me,
                        text_id: app
                            .scenario
                            .strings
                            .iter()
                            .position(|s| s.contains("of 10 Marines"))
                            .map_or(0, |i| i as u32 + 1),
                        kind: 17,
                        unit_id: galaxycraft::ids::unit::MARINE,
                        score_kind: 0,
                        goal: None,
                        location: None,
                    });
                }
                app.draw(&mut fb);
            }
            "countdown" => {
                // Desperate Alliance with 25:43 left (retail frame at 300 s).
                app.load_map(app.map_index);
                app.briefing = false;
                if let Some(t) = app.world.triggers.as_mut() {
                    t.countdown_shown = true;
                    t.countdown_timer = 1543;
                }
                app.draw(&mut fb);
            }
            "game-error" => {
                app.load_map(app.map_index);
                app.briefing = false;
                app.show_sim_error(galaxycraft::sim::SimError::NotEnoughMinerals);
                app.draw(&mut fb);
            }
            "trigger-text" | "trigger-objective" => {
                // The map's own strings, as the trigger shows them.
                app.load_map(app.map_index);
                app.briefing = false;
                let want = if shot_screen == "trigger-text" {
                    "Howdy"
                } else {
                    "NEW MISSION OBJECTIVE"
                };
                if let Some(raw) = app
                    .scenario
                    .strings
                    .iter()
                    .find(|t| t.contains(want))
                    .cloned()
                {
                    app.trigger_text.push(&raw, 0, 60_000);
                }
                app.draw(&mut fb);
            }
            "win-dialog" | "lose-dialog" => {
                app.load_map(app.map_index);
                app.briefing = false;
                app.intro = false;
                app.menu = false;
                app.game_result = Some(shot_screen == "win-dialog");
                app.end_dialog = true;
                app.draw(&mut fb);
            }
            s if s.starts_with("mp-") || s == "score-defeat" || s == "score-victory" => {
                app.mp_shot(s, &mut fb);
            }
            "score" | "victory" | "defeat" => {
                app.load_map(app.map_index);
                app.briefing = false;
                app.intro = false;
                app.menu = false;
                app.game_result = Some(shot_screen != "defeat");
                app.update();
                app.draw(&mut fb);
            }
            "leaderboard" => {
                app.load_map(app.map_index);
                app.briefing = false;
                app.intro = false;
                app.menu = false;
                app.show_leaderboard = true;
                app.update();
                app.draw(&mut fb);
            }
            "stats" => {
                app.load_map(app.map_index);
                app.briefing = false;
                app.intro = false;
                app.menu = false;
                app.show_stats = true;
                app.update();
                app.draw(&mut fb);
            }
            "selection" | "hud" => {
                app.load_map(app.map_index);
                app.briefing = false;
                app.intro = false;
                app.menu = false;
                if let Some(id) = app
                    .world
                    .units
                    .iter()
                    .find(|u| u.alive && u.owner as usize == app.local_player)
                    .map(|u| u.id)
                {
                    app.selected = vec![id];
                    if let Some(u) = app.world.unit(id) {
                        app.camera = (u.x / 256 - app.view.0 / 2, u.y / 256 - app.view.1 / 2);
                        app.clamp_camera();
                    }
                }
                app.refresh_commands();
                app.update();
                app.draw(&mut fb);
            }
            _ => {
                app.load_map(app.map_index);
                app.briefing = false;
                app.intro = false;
                app.menu = false;
                app.update();
                app.draw(&mut fb);
            }
        }
        app.overlay = match shot_screen.as_str() {
            "objectives" => 1,
            "tips" => 2,
            "gamemenu" => 3,
            _ => 0,
        };
        // `GC_BENCH=N`: run N frames of the live loop (update + draw) on the
        // captured screen and report the frame cost (in-game performance).
        if let Some(n) = std::env::var("GC_BENCH")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
        {
            let (mut upd, mut drw, mut worst) = (0.0f64, 0.0f64, 0.0f64);
            for _ in 0..n {
                let t0 = Instant::now();
                app.update();
                let t1 = Instant::now();
                app.draw(&mut fb);
                let t2 = Instant::now();
                upd += (t1 - t0).as_secs_f64() * 1000.0;
                drw += (t2 - t1).as_secs_f64() * 1000.0;
                worst = worst.max((t2 - t0).as_secs_f64() * 1000.0);
            }
            eprintln!(
                "bench {shot_screen} {}x{}: {n} frames, update {:.2} ms, draw {:.2} ms, worst {:.2} ms, tick {}",
                fb.width,
                fb.height,
                upd / n.max(1) as f64,
                drw / n.max(1) as f64,
                worst,
                app.world.tick
            );
        }
        apply_capture_correction(&mut fb);
        if let Some(path) = &shot {
            let composed = presented(&fb, width, height, app.present_offset);
            let fb = composed.as_ref().unwrap_or(&fb);
            match fb.write_bmp(path) {
                Ok(()) => eprintln!(
                    "screenshot: {} ({}x{})",
                    path.display(),
                    fb.width,
                    fb.height
                ),
                Err(e) => {
                    eprintln!("cannot write {}: {e}", path.display());
                    std::process::exit(1);
                }
            }
            return;
        }
    }

    if freeze {
        app.speed = 0.0;
    }
    // Display backend: minifb opens Wayland when the session has it and
    // falls back to X11; `--display x11` hides the Wayland socket so X11
    // (or XWayland) is used, `--display wayland` refuses the fallback.
    match DisplayBackend::parse(display_backend.as_deref()) {
        Ok(DisplayBackend::X11) => {
            // SAFETY: single-threaded here; no other thread reads the
            // environment before the window exists.
            unsafe { std::env::remove_var("WAYLAND_DISPLAY") };
        }
        Ok(DisplayBackend::Wayland) if std::env::var_os("WAYLAND_DISPLAY").is_none() => {
            eprintln!("--display wayland: no Wayland session (WAYLAND_DISPLAY is unset)");
            std::process::exit(1);
        }
        Ok(_) => {}
        Err(other) => {
            eprintln!("--display {other}: expected wayland, x11 or auto");
            std::process::exit(2);
        }
    }
    let mut window = match Window::new(
        "GalaxyCraft - reconstructed StarCraft",
        width,
        height,
        WindowOptions {
            // Resizable even when fullscreen: the compositor / window
            // manager gives the window the monitor size, and every screen
            // follows the window size (letterboxed 16:9).
            resize: true,
            borderless: false,
            ..WindowOptions::default()
        },
    ) {
        Ok(w) => w,
        Err(e) => {
            eprintln!(
                "cannot open a window ({e}); a Wayland or X11 session is needed \
                 (WAYLAND_DISPLAY / DISPLAY)"
            );
            std::process::exit(1);
        }
    };
    eprintln!("display: {}", window.backend());
    // Native fullscreen request (Wayland xdg-shell / X11 EWMH): it keeps
    // keyboard and mouse focus, unlike an override-redirect window.  Sent
    // again once the first frame is up, for window managers that ignore a
    // request made before the window is mapped.
    let mut is_fullscreen = fullscreen;
    window.set_fullscreen(is_fullscreen);
    let mut fullscreen_resent = !fullscreen;
    // The retail front-end shows its own pointer art; hide the system cursor
    // when that art is available so only one arrow is on screen.
    window.set_cursor_visibility(app.system_cursor_visible());
    window.set_target_fps(target_fps(&app.game_options));

    let mut fps_frames = 0u32;
    let mut fps_last = Instant::now();
    let mut draw_ms = 0.0f64;
    let mut present_ms = 0.0f64;
    let mut needs_centre = start_screen.is_some();
    let mut window_size = (fb.width as usize, fb.height as usize);
    let mut window_pixels: Vec<u32> = Vec::new();
    while window.is_open() && !app.quit {
        window.set_target_fps(target_fps(&app.game_options));
        window.set_cursor_visibility(app.system_cursor_visible());
        let (w, h) = window.get_size();
        let (w, h) = (w.max(640), h.max(360));
        let (iw, ih, ox, oy) = letterbox(w, h);
        if iw != fb.width as usize || ih != fb.height as usize || (w, h) != window_size {
            eprintln!("window: {w}x{h}");
            fb = Framebuffer::new(iw, ih);
            app.present_offset = (ox, oy);
            window_size = (w, h);
            app.virtual_pointer = None;
            app.last_raw_pointer = None;
            app.update_view(&fb);
        }
        let raw_pointer = app.surface_mouse(&window);
        app.update_pointer(raw_pointer, (fb.width, fb.height));
        // Mouse edge scrolling: only with the pointer inside the focused
        // window (a clamped position outside it must not scroll).
        app.edge_scroll = app
            .pointer
            .filter(|_| window.is_active() && window.get_mouse_pos(MouseMode::Discard).is_some())
            .and_then(|p| edge_scroll_dir(p, (fb.width, fb.height)));
        if needs_centre {
            // The logical view is only known once the window exists; centring
            // earlier leaves the camera clamped to the map corner (black fog).
            app.centre_on_local_unit();
            app.world.update_vision();
            needs_centre = false;
        }
        if !fullscreen_resent && fps_frames >= 2 {
            fullscreen_resent = true;
            window.set_fullscreen(is_fullscreen);
        }
        let alt = window.is_key_down(Key::LeftAlt) || window.is_key_down(Key::RightAlt);
        // Alt+Enter toggles fullscreen (retail F11 opens the Help Menu).
        if alt && window.is_key_pressed(Key::Enter, KeyRepeat::No) {
            is_fullscreen = !is_fullscreen;
            window.set_fullscreen(is_fullscreen);
        }
        app.handle_input(&window);
        window.set_cursor_visibility(app.system_cursor_visible());
        app.update_audio_background(window.is_active());
        if let Some(full) = app.window_fullscreen.take() {
            is_fullscreen = full;
            window.set_fullscreen(full);
        }
        app.update();
        let draw_started = Instant::now();
        app.draw(&mut fb);
        if app.debug_input {
            app.draw_input_debug(&mut fb);
        }
        if app.capture_request {
            app.capture_request = false;
            let path = std::env::var("GC_CAPTURE_PATH")
                .unwrap_or_else(|_| "/tmp/opencode/menu-shots/live-capture.bmp".to_string());
            let _ = fb.write_bmp(std::path::Path::new(&path));
        }
        app.draw_cursor(&mut fb);
        let present_started = Instant::now();
        if app.present_offset == (0, 0) && window_size == (fb.width as usize, fb.height as usize) {
            window
                .update_with_buffer(&fb.pixels, fb.width as usize, fb.height as usize)
                .expect("present frame");
        } else {
            present_into(
                &fb,
                window_size.0,
                window_size.1,
                app.present_offset,
                &mut window_pixels,
            );
            window
                .update_with_buffer(&window_pixels, window_size.0, window_size.1)
                .expect("present frame");
        }
        let now = Instant::now();
        draw_ms += (present_started - draw_started).as_secs_f64() * 1000.0;
        present_ms += (now - present_started).as_secs_f64() * 1000.0;
        fps_frames += 1;
        let elapsed = now.duration_since(fps_last).as_secs_f64();
        if elapsed >= 1.0 {
            if std::env::var_os("GC_FPS").is_some() {
                eprintln!(
                    "fps {:.1} | draw {:.1} ms | present {:.1} ms",
                    fps_frames as f64 / elapsed,
                    draw_ms / fps_frames as f64,
                    present_ms / fps_frames as f64
                );
            }
            app.fps = (fps_frames as f64 / elapsed) as f32;
            fps_frames = 0;
            draw_ms = 0.0;
            present_ms = 0.0;
            fps_last = now;
        }
    }
    app.flush_profiles();
}

/// Clear the whole menu frame, including the ultrawide pillarbox bars that
/// the 16:9 content box never touches.
fn clear_menu_frame(fb: &mut Framebuffer) {
    fb.clear(Rgb::new(0, 0, 0));
}

/// Collections plate fill: the CEF `rgba(0,0,0,.5)` (the old 224 hid the
/// menu that shows through on the retail panel).
const COLLECTIONS_PLATE_ALPHA: u8 = 128;

/// Whether a computer slot gets the skirmish AI.  Missions (worlds whose
/// victory comes from triggers) drive their computers only through the
/// map's `Run AI Script` actions, as the retail campaign does; a "Rescue
/// Passive" slot never has an AI (its units wait to be rescued).
fn skirmish_ai(world: &World, slot: usize) -> bool {
    !world.trigger_victory && !world.players.get(slot).is_some_and(|p| p.rescuable)
}

/// Bank cue of the rescue sound for a race.
fn rescue_cue(race: Race) -> &'static str {
    match race {
        Race::Zerg => "rescue_zerg",
        Race::Protoss => "rescue_protoss",
        _ => "rescue_terran",
    }
}

/// Greedy word wrap: lines no wider than `max_w` as measured by `measure`
/// (a single word wider than the line keeps its own line).
fn wrap_words(text: &str, max_w: i32, mut measure: impl FnMut(&str) -> i32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let candidate = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if !line.is_empty() && measure(&candidate) > max_w {
            lines.push(std::mem::replace(&mut line, word.to_string()));
        } else {
            line = candidate;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// A static translucent layer captured from two renders (over black `B` and
/// over white `W`): drawing it over any `dst` is, per channel,
/// `dst * (W - B) / 255 + B`.  Only the rows/columns the layer touches are
/// kept.
struct LayerCache {
    x0: i32,
    y0: i32,
    w: i32,
    /// Per pixel: `B` (0x00RRGGBB) and the per-channel keep factor `W - B`.
    base: Vec<u32>,
    keep: Vec<u32>,
}

impl LayerCache {
    fn new(over_black: &Framebuffer, over_white: &Framebuffer) -> Self {
        let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, -1, -1);
        for y in 0..over_black.height {
            for x in 0..over_black.width {
                let i = (y * over_black.width + x) as usize;
                if over_black.pixels[i] & 0xff_ffff != 0
                    || over_white.pixels[i] & 0xff_ffff != 0xff_ffff
                {
                    (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x), y1.max(y));
                }
            }
        }
        if x1 < 0 {
            return Self {
                x0: 0,
                y0: 0,
                w: 0,
                base: Vec::new(),
                keep: Vec::new(),
            };
        }
        let (w, h) = (x1 - x0 + 1, y1 - y0 + 1);
        let mut base = Vec::with_capacity((w * h) as usize);
        let mut keep = Vec::with_capacity((w * h) as usize);
        for y in y0..=y1 {
            for x in x0..=x1 {
                let i = (y * over_black.width + x) as usize;
                let (b, wv) = (over_black.pixels[i], over_white.pixels[i]);
                base.push(b & 0xff_ffff);
                let ch = |s: u32| ((wv >> s) & 0xff).saturating_sub((b >> s) & 0xff);
                keep.push((ch(16) << 16) | (ch(8) << 8) | ch(0));
            }
        }
        Self {
            x0,
            y0,
            w,
            base,
            keep,
        }
    }

    fn apply(&self, fb: &mut Framebuffer) {
        if self.w == 0 {
            return;
        }
        let stride = fb.width as usize;
        for (r, (base, keep)) in self
            .base
            .chunks(self.w as usize)
            .zip(self.keep.chunks(self.w as usize))
            .enumerate()
        {
            let y = self.y0 + r as i32;
            if y < 0 || y >= fb.height {
                continue;
            }
            let row = &mut fb.pixels[y as usize * stride..(y as usize + 1) * stride];
            for (i, (&b, &k)) in base.iter().zip(keep).enumerate() {
                let x = self.x0 + i as i32;
                if x < 0 || x >= fb.width {
                    continue;
                }
                // Opaque and fully transparent texels need no arithmetic.
                if k == 0 {
                    row[x as usize] = b;
                    continue;
                }
                if k == 0xff_ffff && b == 0 {
                    continue;
                }
                let d = row[x as usize];
                let ch = |s: u32| {
                    (((d >> s) & 0xff) * ((k >> s) & 0xff) + 127) / 255 + ((b >> s) & 0xff)
                };
                row[x as usize] = (ch(16).min(255) << 16) | (ch(8).min(255) << 8) | ch(0).min(255);
            }
        }
    }
}

/// Run a screen that lays itself out over the whole frame (briefing pages,
/// the score screen) inside the 16:9 content box: it draws into a box-sized
/// framebuffer that is then copied centred, so an ultrawide window shows
/// exactly the 16:9 picture between black bars, as the retail client does.
fn draw_in_box(app: &mut App, fb: &mut Framebuffer, draw: fn(&mut App, &mut Framebuffer)) {
    let (x0, y0, w, h) = App::content_rect(fb);
    if x0 == 0 && y0 == 0 && w == fb.width && h == fb.height {
        draw(app, fb);
        return;
    }
    let mut boxed = match app.boxed_fb.take() {
        Some(buffer) if buffer.width == w && buffer.height == h => buffer,
        _ => Framebuffer::new(w as usize, h as usize),
    };
    draw(app, &mut boxed);
    copy_into(fb, &boxed, x0, y0);
    app.boxed_fb = Some(boxed);
}

/// Copy `src` into `dst` with its top-left corner at `(x0, y0)`.
fn copy_into(dst: &mut Framebuffer, src: &Framebuffer, x0: i32, y0: i32) {
    for y in 0..src.height {
        let dy = y + y0;
        if dy < 0 || dy >= dst.height {
            continue;
        }
        let left = x0.max(0);
        let right = (x0 + src.width).min(dst.width);
        if right <= left {
            continue;
        }
        let s0 = (y * src.width + (left - x0)) as usize;
        let d0 = (dy * dst.width + left) as usize;
        let n = (right - left) as usize;
        dst.pixels[d0..d0 + n].copy_from_slice(&src.pixels[s0..s0 + n]);
    }
}

/// Alpha-blend a solid rectangle over the framebuffer (controller wheel and
/// hint chrome; the rest of the HUD uses the shipped plate cache).
/// Soft-filled circle.  Used by the command wheel's central plate and icon
/// discs, which must darken the playfield without hard aliased edges.
fn blend_circle(fb: &mut Framebuffer, cx: i32, cy: i32, radius: i32, color: Rgb, alpha: u8) {
    let r = radius.max(0);
    for dy in -r..=r {
        let half = ((r * r - dy * dy).max(0) as f32).sqrt() as i32;
        for dx in -half..=half {
            let c = fb.get(cx + dx, cy + dy);
            fb.put(cx + dx, cy + dy, c.blend(color, alpha));
        }
    }
}

/// Black side bars outside the 16:9 content box `(x0, w)`.  Every retail
/// front-end capture at 3440x1440 (all 56 references) is black there
/// (max channel <= 8): no plate, arm or backdrop reaches the bars.
fn mask_pillarbox(fb: &mut Framebuffer, x0: i32, w: i32) {
    let black = Rgb::new(0, 0, 0);
    let right = x0 + w;
    for y in 0..fb.height {
        if x0 > 0 {
            fb.rect(0, y, x0, 1, black);
        }
        if right < fb.width {
            fb.rect(right, y, fb.width - right, 1, black);
        }
    }
}

/// `A`..`Z` as typed in the rebind input field.
/// The key that types `c` (letters with or without Shift, digits, space).
fn char_key(c: char) -> Option<Key> {
    const LETTERS: [Key; 26] = [
        Key::A,
        Key::B,
        Key::C,
        Key::D,
        Key::E,
        Key::F,
        Key::G,
        Key::H,
        Key::I,
        Key::J,
        Key::K,
        Key::L,
        Key::M,
        Key::N,
        Key::O,
        Key::P,
        Key::Q,
        Key::R,
        Key::S,
        Key::T,
        Key::U,
        Key::V,
        Key::W,
        Key::X,
        Key::Y,
        Key::Z,
    ];
    const DIGITS: [Key; 10] = [
        Key::Key0,
        Key::Key1,
        Key::Key2,
        Key::Key3,
        Key::Key4,
        Key::Key5,
        Key::Key6,
        Key::Key7,
        Key::Key8,
        Key::Key9,
    ];
    match c {
        'a'..='z' | 'A'..='Z' => Some(LETTERS[(c.to_ascii_uppercase() as u8 - b'A') as usize]),
        '0'..='9' => Some(DIGITS[(c as u8 - b'0') as usize]),
        ' ' => Some(Key::Space),
        _ => None,
    }
}

fn key_letter(key: Key) -> Option<char> {
    Some(match key {
        Key::A => 'A',
        Key::B => 'B',
        Key::C => 'C',
        Key::D => 'D',
        Key::E => 'E',
        Key::F => 'F',
        Key::G => 'G',
        Key::H => 'H',
        Key::I => 'I',
        Key::J => 'J',
        Key::K => 'K',
        Key::L => 'L',
        Key::M => 'M',
        Key::N => 'N',
        Key::O => 'O',
        Key::P => 'P',
        Key::Q => 'Q',
        Key::R => 'R',
        Key::S => 'S',
        Key::T => 'T',
        Key::U => 'U',
        Key::V => 'V',
        Key::W => 'W',
        Key::X => 'X',
        Key::Y => 'Y',
        Key::Z => 'Z',
        _ => return None,
    })
}

/// A character typed into the in-game chat line: letters and digits in
/// lower case.  Space, Enter, Backspace and Escape have their own actions.
fn chat_char(key: Key) -> Option<char> {
    key_letter(key)
        .or_else(|| mp::key_digit(key))
        .map(|c| c.to_ascii_lowercase())
}

/// CEF `.unit.bindingError:after`: 30 px rgba(0,0,0,.6) plate and the 24 px
/// `unbound.png` red disc with a transparent question mark.
fn draw_unbound_badge(fb: &mut Framebuffer, bx: i32, by: i32, dx: i32, dy: i32) {
    fill_alpha_rect(fb, bx + dx * 4, by + dy * 3, dx * 30, dy * 30, 153);
    let cx = bx as f32 + dx as f32 * 19.0;
    let cy = by as f32 + dy as f32 * 18.0;
    let r = dx as f32 * 12.0;
    let red = Rgb::new(204, 14, 0);
    for yy in (cy - r - 1.0) as i32..(cy + r + 1.0) as i32 {
        for xx in (cx - r - 1.0) as i32..(cx + r + 1.0) as i32 {
            let dist = ((xx as f32 + 0.5 - cx).powi(2) + (yy as f32 + 0.5 - cy).powi(2)).sqrt();
            let cov = (r + 0.5 - dist).clamp(0.0, 1.0);
            if cov > 0.0 {
                let under = fb.get(xx, yy);
                fb.put(xx, yy, under.blend(red, (cov * 255.0) as u8));
            }
        }
    }
}

impl App {}

impl App {}

/// Display backend asked with `--display` / `GALAXYCRAFT_DISPLAY`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DisplayBackend {
    /// Wayland when the session has it, else X11 (minifb's own order).
    Auto,
    Wayland,
    /// X11, or XWayland inside a Wayland session.
    X11,
}

impl DisplayBackend {
    fn parse(value: Option<&str>) -> Result<Self, String> {
        match value.map(|v| v.trim().to_ascii_lowercase()).as_deref() {
            None | Some("") | Some("auto") => Ok(Self::Auto),
            Some("wayland") => Ok(Self::Wayland),
            Some("x11") | Some("xorg") | Some("xwayland") => Ok(Self::X11),
            Some(other) => Err(other.to_string()),
        }
    }
}

#[cfg(test)]
mod display_tests {
    use super::DisplayBackend;

    #[test]
    fn show_fps_label_rounds_the_measured_rate() {
        assert_eq!(super::fps_label(59.6), "FPS 60");
        assert_eq!(super::fps_label(0.0), "FPS 0");
    }

    #[test]
    fn building_sound_option_gates_only_structures() {
        use galaxycraft::menu::GameOptions;
        let mut options = GameOptions::default();
        assert!(super::building_sound_allowed(&options, true));
        assert!(super::building_sound_allowed(&options, false));
        options.building_sounds = false;
        assert!(!super::building_sound_allowed(&options, true));
        assert!(super::building_sound_allowed(&options, false));
    }

    #[test]
    fn turn_rate_label_shows_the_selected_flingy_value() {
        assert_eq!(super::turn_rate_label(Some(40)), "TURN 40");
        assert_eq!(super::turn_rate_label(None), "TURN -");
    }

    #[test]
    fn collections_console_cells_map_to_the_three_skins() {
        assert_eq!(super::collection_console_skin(0), Some(0));
        assert_eq!(super::collection_console_skin(1), Some(1));
        assert_eq!(super::collection_console_skin(2), Some(2));
        assert_eq!(super::collection_console_skin(3), None, "unit skin cell");
        assert_eq!(super::collection_console_skin(5), None, "announcer cell");
        assert_eq!(super::collection_console_skin(10), None);
    }

    #[test]
    fn collections_announcer_cells_map_to_the_six_sets() {
        assert_eq!(super::collection_announcer(5), Some(0), "Default");
        assert_eq!(super::collection_announcer(6), Some(1), "Um");
        assert_eq!(super::collection_announcer(7), Some(2), "Jeon");
        assert_eq!(super::collection_announcer(8), Some(3), "Kim");
        assert_eq!(super::collection_announcer(9), Some(4), "UmJeonKim");
        assert_eq!(super::collection_announcer(10), Some(5), "HeyJini");
        assert_eq!(super::collection_announcer(4), None);
        assert_eq!(super::collection_announcer(11), None);
    }

    #[test]
    fn presale_bonus_skins_cover_the_shipped_units() {
        use galaxycraft::ids::unit as uid;
        let options = galaxycraft::menu::GameOptions::default();
        assert_eq!(super::unit_skin_id(uid::COMMAND_CENTER, &options), 1);
        assert_eq!(super::unit_skin_id(uid::NEXUS, &options), 1);
        assert_eq!(super::unit_skin_id(uid::SCOURGE, &options), 1);
        let options = galaxycraft::menu::GameOptions {
            game_bonus_skins: false,
            ..Default::default()
        };
        assert_eq!(super::unit_skin_id(uid::COMMAND_CENTER, &options), 0);
        assert_eq!(super::unit_skin_id(uid::MARINE, &options), 0);
    }

    #[test]
    fn carbot_collection_skins_every_unit() {
        use galaxycraft::ids::unit as uid;
        let options = galaxycraft::menu::GameOptions {
            game_unit_skin: 2,
            ..Default::default()
        };
        for unit in [uid::MARINE, uid::COMMAND_CENTER, uid::ZERGLING, uid::ZEALOT] {
            assert_eq!(super::unit_skin_id(unit, &options), 2);
        }
        let options = galaxycraft::menu::GameOptions {
            game_bonus_skins: false,
            ..Default::default()
        };
        assert_eq!(super::unit_skin_id(uid::MARINE, &options), 0);
    }

    #[test]
    fn mouse_sensitivity_and_scaling_scale_the_pointer() {
        use galaxycraft::menu::GameOptions;
        let mut o = GameOptions::default();
        assert_eq!(super::mouse_move_factor(&o, (1280, 720)), 1.0);
        o.mouse_sensitivity = 1.0;
        assert_eq!(super::mouse_move_factor(&o, (1280, 720)), 1.5);
        o.use_mouse_sensitivity = false;
        assert_eq!(super::mouse_move_factor(&o, (1280, 720)), 1.0);
        o.mouse_scaling = true;
        assert_eq!(super::mouse_move_factor(&o, (1280, 720)), 1.5);
        assert_eq!(super::mouse_move_factor(&o, (640, 480)), 1.0);
    }

    #[test]
    fn virtual_pointer_advances_by_the_scaled_delta_and_clamps() {
        let start = (10.0, 10.0);
        let next = super::advance_virtual_pointer(start, start, (20.0, 14.0), 0.5, (100, 80));
        assert_eq!(next, (15.0, 12.0));
        let edge =
            super::advance_virtual_pointer((95.0, 5.0), (95.0, 5.0), (120.0, 5.0), 1.0, (100, 80));
        assert_eq!(edge.0, 99.0);
    }

    #[test]
    fn standard_minimap_position_moves_towards_the_centre() {
        // 2560 content at s=3 is a 853.3 game-px playfield; the classic
        // 640 px console leaves 106.7 game px on each side.
        assert_eq!(super::minimap_centering_shift(0, "minimap", 2560, 3.0), 0.0);
        let shift = super::minimap_centering_shift(1, "minimap", 2560, 3.0);
        assert!((shift - 106.67).abs() < 0.1, "shift {shift}");
        assert_eq!(
            super::minimap_centering_shift(1, "statdata", 2560, 3.0),
            0.0
        );
        // 4:3 has no spare width.
        assert_eq!(super::minimap_centering_shift(1, "minimap", 1920, 3.0), 0.0);
    }

    #[test]
    fn sd_graphics_filter_chooses_nearest_or_bilinear() {
        use super::{Framebuffer, Rgb};
        let mut src = Framebuffer::new(2, 2);
        let red = Rgb::new(255, 0, 0);
        let green = Rgb::new(0, 255, 0);
        let blue = Rgb::new(0, 0, 255);
        let white = Rgb::new(255, 255, 255);
        src.put(0, 0, red);
        src.put(1, 0, green);
        src.put(0, 1, blue);
        src.put(1, 1, white);
        let mut sharp = Framebuffer::new(4, 4);
        super::blit_scaled_at(&src, &mut sharp, 2.0, 0, 0.0);
        assert_eq!(sharp.get(0, 0), red);
        assert_eq!(sharp.get(1, 1), red, "nearest keeps 2x2 texel blocks");
        assert_eq!(sharp.get(2, 0), green);
        let mut filtered = Framebuffer::new(4, 4);
        super::blit_scaled_at(&src, &mut filtered, 2.0, 0, 1.0);
        assert_eq!(filtered.get(0, 0), red);
        assert_ne!(filtered.get(1, 1), red, "filtered stops interpolate");
        assert_ne!(filtered.get(1, 1), filtered.get(2, 2));
    }

    #[test]
    fn pillarbox_uses_a_centred_four_by_three_content_box() {
        use super::{App, Framebuffer, GAME_PILLARBOX, Ordering};
        let fb = Framebuffer::new(1920, 1080);
        GAME_PILLARBOX.store(false, Ordering::Relaxed);
        assert_eq!(App::content_rect(&fb), (0, 0, 1920, 1080));
        GAME_PILLARBOX.store(true, Ordering::Relaxed);
        assert_eq!(App::content_rect(&fb), (240, 0, 1440, 1080));
        // A narrower window still fits the 4:3 box.
        let fb = Framebuffer::new(1280, 960);
        assert_eq!(App::content_rect(&fb), (0, 0, 1280, 960));
        GAME_PILLARBOX.store(false, Ordering::Relaxed);
    }

    #[test]
    fn mature_language_filter_masks_whole_words_and_keeps_layout() {
        assert_eq!(
            super::mature_language_filter("What the FUCK, shit!"),
            "What the ****, ****!"
        );
        assert_eq!(
            super::mature_language_filter("Classic assistant"),
            "Classic assistant",
            "only whole words are masked"
        );
        assert_eq!(super::mature_language_filter("damn\nhell"), "****\n****");
    }

    #[test]
    fn smooth_turning_rotates_the_short_way_around_the_facing_circle() {
        // 250 -> 10 is +16 through 0, not -240.
        assert_eq!(super::smooth_step_facing(250.0, 10, 8.0), 2.0);
        // And back down the same way.
        assert_eq!(super::smooth_step_facing(10.0, 250, 8.0), 2.0);
        // No target change leaves the facing alone.
        assert_eq!(super::smooth_step_facing(64.0, 64, 24.0), 64.0);
        // 0 -> 200 is -56, so a 20-unit step goes to 236.
        assert_eq!(super::smooth_step_facing(0.0, 200, 20.0), 236.0);
    }

    #[test]
    fn environment_particles_are_deterministic_and_era_specific() {
        let hashes: Vec<u32> = (0..8).map(super::env_hash).collect();
        assert_eq!(hashes, (0..8).map(super::env_hash).collect::<Vec<_>>());
        assert_ne!(hashes[0], hashes[1]);
        assert_ne!(
            super::env_spec(3).colour,
            super::env_spec(6).colour,
            "ash and snow differ"
        );
        assert!(super::env_spec(5).drift < 0.0, "desert sand blows sideways");
        // The shipped particle texture supplies coverage; the era the colour.
        let img = galaxycraft::dds::DdsImage {
            width: 1,
            height: 1,
            pixels: vec![0xff00_0000],
        };
        let mut fb = galaxycraft::render::Framebuffer::new(4, 4);
        fb.clear(galaxycraft::render::Rgb::new(0, 0, 0));
        super::draw_env_particle(
            &mut fb,
            &img,
            1,
            1,
            1,
            galaxycraft::render::Rgb::new(255, 255, 255),
            255,
        );
        assert_eq!(fb.get(1, 1), galaxycraft::render::Rgb::new(255, 255, 255));
        assert_eq!(fb.get(0, 0), galaxycraft::render::Rgb::new(0, 0, 0));
    }

    #[test]
    fn video_options_drive_the_window_polling_rate() {
        use galaxycraft::menu::GameOptions;
        let mut o = GameOptions::default();
        assert_eq!(super::target_fps(&o), 300);
        o.video_fps_limit = 0.0;
        assert_eq!(super::target_fps(&o), 100);
        o.video_fps_cap = false;
        assert_eq!(super::target_fps(&o), 60, "VSync fallback");
        o.video_vsync = false;
        assert_eq!(super::target_fps(&o), 0, "unlimited");
    }

    #[test]
    fn apm_uses_the_mission_clock_and_waits_for_the_first_minute() {
        let minute = galaxycraft::sim::TICKS_PER_SECOND as u64 * 60;
        assert_eq!(super::apm_value(60, minute), 60);
        assert_eq!(super::apm_value(120, minute * 2), 60);
        assert_eq!(super::apm_value(30, minute / 2), 0, "opening minute");
        assert_eq!(super::apm_value(1, 0), 0);
    }

    #[test]
    fn cargo_grid_packs_small_medium_and_large_units_without_overlap() {
        let controls = |sizes: &[u8], capacity| {
            super::cargo_slots(sizes, capacity)
                .into_iter()
                .map(|s| s.control)
                .collect::<Vec<_>>()
        };
        // Eight one-space units keep the retail column-major 25-32 order.
        assert_eq!(controls(&[1; 8], 8), vec![25, 26, 27, 28, 29, 30, 31, 32]);
        // A 2-space unit takes a full 1 x 2 column; the next Marine moves
        // to the next free column.
        assert_eq!(controls(&[2, 1], 8), vec![21, 27]);
        // Two 4-space units fill the left and right 2 x 2 blocks.
        assert_eq!(controls(&[4, 4], 8), vec![18, 19]);
        // After one 4-space block, four Marines use the right half.
        assert_eq!(controls(&[4, 1, 1, 1, 1], 8), vec![18, 29, 30, 31, 32]);
        // A Marine first leaves the left column's top cell taken, so the
        // first 4-space block starts in the middle column.
        assert_eq!(controls(&[1, 4], 8), vec![25, 20]);
        // Bunkers show their four one-space passengers in the centred
        // 2 x 2 block.
        assert_eq!(controls(&[1; 4], 4), vec![27, 28, 29, 30]);
    }

    #[test]
    fn the_score_screen_leaves_like_the_recording() {
        // Backwater Station 651.0-653.0 s: panel out in 0.25 s, backdrop
        // alone, black from 1.5 s, the next screen at 2.0 s.
        assert_eq!(super::score_leave_shift(0.0), 0.0);
        assert!(super::score_leave_shift(0.25) <= -super::SCORE_SLIDE_FROM);
        const { assert!(super::SCORE_LEAVE_BLACK_AT < super::SCORE_LEAVE_SECS) };
        assert!((super::SCORE_LEAVE_SECS - 2.0).abs() < 1e-6);
    }

    #[test]
    fn a_right_click_target_blinks_in_its_owner_colour() {
        // User report 2026-11-07: an SCV sent to minerals shows the yellow
        // circle; resources used to come out red.
        use super::{HUD_SELECT_GREEN, HUD_SELECT_RED, HUD_SELECT_YELLOW, circle_colour_for};
        assert_eq!(
            circle_colour_for(false, true, true),
            HUD_SELECT_YELLOW,
            "mineral"
        );
        assert_eq!(
            circle_colour_for(false, false, true),
            HUD_SELECT_RED,
            "enemy"
        );
        assert_eq!(
            circle_colour_for(true, false, false),
            HUD_SELECT_GREEN,
            "own"
        );
        assert!(super::order_flash_on(0.05));
        assert!(!super::order_flash_on(0.2));
        assert!(super::order_flash_on(0.3));
        assert!(!super::order_flash_on(0.8), "done after three blinks");
    }

    #[test]
    fn classic_glue_labels_drop_hotkey_control_bytes() {
        assert_eq!(
            super::decode_glue_string("s\\u0004S\\u0001ingle Player"),
            "Single Player"
        );
        assert_eq!(
            super::decode_glue_string("iView \\u0004I\\u0001ntro"),
            "View Intro"
        );
        assert_eq!(super::decode_glue_string("\\u0005Exit"), "Exit");
    }

    #[test]
    fn glue_json_lists_decode_hotkey_markup() {
        let json =
            br#"[{"Key":"gluCreat-2","Value":"o\u0004O\u0001k"},{"Key":"empty","Value":""}]"#;
        let strings = super::parse_json_strings(json);
        assert_eq!(strings.get("gluCreat-2").map(String::as_str), Some("Ok"));
        assert_eq!(strings.get("empty").map(String::as_str), Some(""));
    }

    #[test]
    fn a_typed_save_name_becomes_a_safe_file_name() {
        assert_eq!(
            super::save_file_stem("Este es un save (Boot Camp)"),
            "Este es un save (Boot Camp)"
        );
        assert_eq!(super::save_file_stem("a/b\\c:d"), "a_b_c_d");
        assert_eq!(super::save_file_stem("  "), "save");
        assert_eq!(super::save_file_stem(".."), "save");
        assert_eq!(super::save_file_stem(&"x".repeat(40)).len(), 32);
    }

    #[test]
    fn the_campaign_list_plates_slide_in_from_both_sides() {
        // User report 2026-11-07 (recording 441.35-441.75 s): the campaign
        // list's console plates slide in like the glue lists'.
        use galaxycraft::menu::MenuScreen;
        assert!(super::slides_in(&MenuScreen::Episode));
        assert!(super::slides_in(&MenuScreen::Registry));
        assert!(!super::slides_in(&MenuScreen::Main));
        // The left plate starts fully off screen and both land in place.
        let off = galaxycraft::menu::swish_in_offset(super::SWISH_EPISODE_LEFT_UI, 0);
        assert_eq!(off, super::SWISH_EPISODE_LEFT_UI);
        let n = galaxycraft::menu::swish_in_steps(super::SWISH_EPISODE_LEFT_UI);
        assert_eq!(
            galaxycraft::menu::swish_in_offset(super::SWISH_EPISODE_RIGHT_UI, n),
            0.0
        );
    }

    #[test]
    fn load_saved_lists_only_saves_without_the_cloud_folder() {
        // User request 2026-11-07: no "[Cloud]" row.
        assert!(super::App::saved_rows(&[], 0).is_empty());
        let rows = super::App::saved_rows(&[std::path::PathBuf::from("/nonexistent/a.gcsv")], 0);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].0.starts_with("a.gcsv"));
        assert!(rows.iter().all(|(r, _)| !r.contains("Cloud")));
    }

    #[test]
    fn the_display_backend_option_accepts_wayland_x11_and_auto() {
        assert_eq!(DisplayBackend::parse(None), Ok(DisplayBackend::Auto));
        assert_eq!(
            DisplayBackend::parse(Some("Wayland")),
            Ok(DisplayBackend::Wayland)
        );
        assert_eq!(DisplayBackend::parse(Some("x11")), Ok(DisplayBackend::X11));
        assert_eq!(
            DisplayBackend::parse(Some("XWayland")),
            Ok(DisplayBackend::X11)
        );
        assert!(DisplayBackend::parse(Some("vulkan")).is_err());
    }
}

#[cfg(test)]
mod timestamp_tests {
    use super::format_timestamp;
    use std::time::{Duration, UNIX_EPOCH};

    #[test]
    fn timestamps_render_as_civil_dates() {
        assert_eq!(format_timestamp(UNIX_EPOCH), "01/01/1970 12:00 AM");
        // 2026-10-29 20:34:00 UTC
        let t = UNIX_EPOCH + Duration::from_secs(1_793_306_040);
        assert_eq!(format_timestamp(t), "10/29/2026 8:34 PM");
        // Leap day, noon.
        let t = UNIX_EPOCH + Duration::from_secs(1_709_164_800 + 12 * 3600);
        assert_eq!(format_timestamp(t), "02/29/2024 12:00 PM");
    }
}

#[cfg(test)]
mod menu_frame_tests {
    use super::{Framebuffer, Rgb, clear_menu_frame};

    #[test]
    fn town_hall_supply_rows_use_the_race_labels() {
        use super::supply_labels;
        use galaxycraft::sim::Race;
        let data = galaxycraft::data::GameData::bundled().expect("bundled data");
        let terran = supply_labels(&data.strings, Race::Terran);
        let zerg = supply_labels(&data.strings, Race::Zerg);
        let protoss = supply_labels(&data.strings, Race::Protoss);
        assert_eq!(terran.len(), 4);
        assert_eq!(zerg.len(), 4);
        assert_eq!(protoss.len(), 4);
        assert!(terran.iter().all(|s| !s.is_empty()));
        assert_ne!(terran[0], zerg[0]);
        assert_ne!(terran[2], protoss[2]);
    }

    #[test]
    fn refused_orders_map_to_the_retail_error_rows() {
        use super::stat_error_id;
        use galaxycraft::sim::{Race, SimError};
        assert_eq!(
            stat_error_id(SimError::NotEnoughMinerals, Race::Terran),
            Some(850)
        );
        assert_eq!(stat_error_id(SimError::NotEnoughGas, Race::Zerg), Some(851));
        assert_eq!(
            stat_error_id(SimError::NotEnoughSupply, Race::Terran),
            Some(845)
        );
        assert_eq!(
            stat_error_id(SimError::NotEnoughSupply, Race::Protoss),
            Some(846)
        );
        assert_eq!(
            stat_error_id(SimError::NotEnoughSupply, Race::Zerg),
            Some(844)
        );
        assert_eq!(
            stat_error_id(SimError::NotEnoughEnergy, Race::Terran),
            Some(864)
        );
        assert_eq!(stat_error_id(SimError::Unsupported, Race::Terran), None);
        assert_eq!(super::STAT_CANT_BUILD_THERE, 856);
    }

    #[test]
    fn ready_text_scroll_follows_the_retail_recording() {
        use super::{READY_TEXT_PITCH, ready_text_scroll};
        // Tracked offsets (px) on the Wasteland recording, 0.1 s after
        // each line started: the Adjutant (11 lines, 26 469 ms) and Duke
        // (9 lines, 27 794 ms).
        let adj = 11.0 * READY_TEXT_PITCH;
        let duke = 9.0 * READY_TEXT_PITCH;
        for (t, want) in [
            (2_500, 0.0),
            (6_500, 45.0),
            (12_500, 153.0),
            (22_500, 336.0),
        ] {
            let got = ready_text_scroll(t, 26_469, adj);
            assert!((got - want).abs() <= 12.0, "adjutant {t}: {got} vs {want}");
        }
        for (t, want) in [
            (2_500, 0.0),
            (6_500, 33.0),
            (14_500, 138.0),
            (26_500, 300.0),
        ] {
            let got = ready_text_scroll(t, 27_794, duke);
            assert!((got - want).abs() <= 12.0, "duke {t}: {got} vs {want}");
        }
        // Boot Camp's 4-line welcome is still at 3 s (retail screenshot).
        assert_eq!(ready_text_scroll(2_000, 8_449, 4.0 * READY_TEXT_PITCH), 0.0);
    }

    #[test]
    fn countdown_reads_minutes_and_seconds() {
        use super::countdown_text;
        assert_eq!(countdown_text(1543), "25:43");
        assert_eq!(countdown_text(204), "3:24");
        assert_eq!(countdown_text(0), "0:00");
    }

    #[test]
    fn score_count_up_follows_the_retail_curve() {
        use super::{score_count, score_panel_shift};
        // Overview "Units" (900) on the recording, 0.1 s apart from
        // 8.4 s (count start 8.375 s): 112, 437, 627, 739, 804, 843, 865.
        let seen = [112, 437, 627, 739, 804, 843, 865];
        for (i, want) in seen.iter().enumerate() {
            let got = score_count(900, 0.025 + 0.1 * i as f32);
            assert!((got - want).abs() <= 12, "{i}: {got} vs {want}");
        }
        assert_eq!(score_count(900, 0.0), 0);
        assert_eq!(score_count(900, 2.0), 900, "settles on the value");
        // Panel right edge (2187 content px at rest) at 7.8 / 7.9 / 8.0 s
        // = 0.8 / 0.9 / 1.0 s after the backdrop: 1653 / 1959 / 2088.
        for (t, edge) in [(0.8, 1653.0), (0.9, 1959.0), (1.0, 2088.0)] {
            let got = 2187.0 + score_panel_shift(t);
            assert!((got - edge).abs() < 25.0, "{t}: {got} vs {edge}");
        }
        assert!(score_panel_shift(0.3) < -2000.0, "not in yet");
        assert_eq!(score_panel_shift(2.0), 0.0);
    }

    #[test]
    fn score_columns_follow_the_retail_tabs() {
        use super::ScoreRow;
        let mut p = galaxycraft::sim::Player::new(galaxycraft::sim::Race::Terran, 0, 0, 0);
        p.scores = [0, 850, 525, 0, 50, 0, 0, 0];
        p.minerals_gathered = 776;
        (p.units_produced, p.kills, p.razings, p.units_lost) = (17, 3, 0, 0);
        (p.structures_built, p.minerals_spent) = (3, 400);
        let row = ScoreRow {
            slot: 1,
            human: true,
            name: "LinKa".into(),
            force: "Colonial Militia".into(),
            apm: Some(13),
            player: p,
        };
        // Overview: Units, Structures, Resources and their sum (retail
        // Wasteland: 900 + 525 + 776 = 2201).
        assert_eq!(row.columns(0), [900, 525, 776, 2201]);
        assert_eq!(row.columns(1), [17, 3, 0, 900]);
        assert_eq!(row.columns(2), [3, 0, 0, 525]);
        assert_eq!(row.columns(3), [0, 776, 400, 776]);
    }

    #[test]
    fn layer_cache_reproduces_a_translucent_draw() {
        use super::LayerCache;
        // A 50 % black band over the middle row and an opaque red pixel.
        let draw = |fb: &mut Framebuffer| {
            for x in 0..4 {
                let c = fb.get(x, 1);
                fb.put(x, 1, c.blend(Rgb::new(0, 0, 0), 128));
            }
            fb.put(2, 2, Rgb::new(255, 0, 0));
        };
        let (mut b, mut w) = (Framebuffer::new(4, 3), Framebuffer::new(4, 3));
        b.clear(Rgb::new(0, 0, 0));
        w.clear(Rgb::new(255, 255, 255));
        draw(&mut b);
        draw(&mut w);
        let layer = LayerCache::new(&b, &w);
        for bg in [Rgb::new(10, 200, 90), Rgb::new(250, 3, 128)] {
            let (mut want, mut got) = (Framebuffer::new(4, 3), Framebuffer::new(4, 3));
            want.clear(bg);
            got.clear(bg);
            draw(&mut want);
            layer.apply(&mut got);
            for (p, q) in want.pixels.iter().zip(&got.pixels) {
                for s in [0, 8, 16] {
                    let (a, c) = ((p >> s) & 0xff, (q >> s) & 0xff);
                    assert!(a.abs_diff(c) <= 1, "{p:06x} vs {q:06x}");
                }
            }
        }
    }

    #[test]
    fn wrap_words_keeps_lines_inside_the_width() {
        use super::wrap_words;
        // One "pixel" per character.
        let lines = wrap_words("the quick brown fox jumps", 10, |t| t.len() as i32);
        assert_eq!(lines, ["the quick", "brown fox", "jumps"]);
        assert!(lines.iter().all(|l| l.len() <= 10));
        assert_eq!(
            wrap_words("supercalifragilistic", 5, |t| t.len() as i32).len(),
            1
        );
    }

    #[test]
    fn movies_fill_the_16_9_box() {
        use super::{PosterFrame, draw_movie_frame};
        // A 320x308 SD frame (the Blizzard logo) covers the whole box.
        let frame = PosterFrame {
            width: 320,
            height: 308,
            rgb: vec![200; 320 * 308 * 3],
        };
        let mut fb = Framebuffer::new(3440, 1440);
        draw_movie_frame(&mut fb, &frame);
        for (x, y) in [(440, 0), (2999, 1439), (1720, 720)] {
            assert_eq!(fb.get(x, y), Rgb::new(200, 200, 200), "box pixel ({x},{y})");
        }
        assert_eq!(fb.get(439, 720), Rgb::new(0, 0, 0));
        assert_eq!(fb.get(3000, 720), Rgb::new(0, 0, 0));
    }

    #[test]
    fn boxed_screens_land_centred_between_black_bars() {
        use super::{copy_into, mask_pillarbox};
        let mut fb = Framebuffer::new(3440, 1440);
        fb.clear(Rgb::new(255, 255, 255));
        let mut boxed = Framebuffer::new(2560, 1440);
        boxed.clear(Rgb::new(10, 20, 30));
        copy_into(&mut fb, &boxed, 440, 0);
        mask_pillarbox(&mut fb, 440, 2560);
        assert_eq!(fb.get(439, 700), Rgb::new(0, 0, 0), "left bar");
        assert_eq!(fb.get(440, 700), Rgb::new(10, 20, 30), "box left edge");
        assert_eq!(fb.get(2999, 0), Rgb::new(10, 20, 30), "box right edge");
        assert_eq!(fb.get(3000, 1439), Rgb::new(0, 0, 0), "right bar");
    }

    #[test]
    fn menu_frame_clears_ultrawide_pillarbox_and_cursor_trails() {
        let mut fb = Framebuffer::new(3440, 1440);
        fb.clear(Rgb::new(255, 255, 255));
        clear_menu_frame(&mut fb);
        for (x, y) in [(0, 0), (439, 1439), (3000, 0), (3439, 1439), (1720, 720)] {
            assert_eq!(
                fb.get(x, y),
                Rgb::new(0, 0, 0),
                "stale pixel at ({x},{y}) in the menu frame"
            );
        }
    }
}

#[cfg(test)]
mod hunters_pose_tests {
    use super::{Race, hunters_poses};
    use galaxycraft::ids::unit as u;

    #[test]
    fn every_start_worker_has_a_recorded_pose_on_the_32_direction_grid() {
        for (race, worker) in [
            (Race::Protoss, u::PROBE),
            (Race::Terran, u::SCV),
            (Race::Zerg, u::DRONE),
        ] {
            let poses = hunters_poses(race);
            assert_eq!(
                poses.iter().filter(|p| p.0 == worker).count(),
                4,
                "{race:?}"
            );
            assert!(
                poses.iter().all(|p| p.3 % 8 == 0),
                "{race:?} facings are HD directions"
            );
        }
    }
}

#[cfg(test)]
mod drag_box_tests {
    use super::drag_box;

    #[test]
    fn the_selection_box_spans_the_drag_in_window_pixels() {
        assert_eq!(drag_box(None, (5, 5)), None);
        assert_eq!(
            drag_box(Some((40, 30)), (40, 30)),
            None,
            "a click draws none"
        );
        assert_eq!(
            drag_box(Some((900, 500)), (300, 200)),
            Some((300, 200, 600, 300))
        );
    }
}

#[cfg(test)]
mod tile_grid_tests {
    use super::{HUNTERS_ZERG_CREEP, parse_tile_grid};

    #[test]
    fn hex_grid_maps_digits_to_tiles_and_skips_dots() {
        let g = parse_tile_grid((10, 20), &["0.a", ".c."]);
        assert_eq!(g.get(&(10, 20)), Some(&0));
        assert_eq!(g.get(&(12, 20)), Some(&10));
        assert_eq!(g.get(&(11, 21)), Some(&12));
        assert_eq!(g.len(), 3);
        // The recorded capture only uses the 13 creep subtiles.
        let creep = parse_tile_grid(HUNTERS_ZERG_CREEP.0, HUNTERS_ZERG_CREEP.1);
        assert!(creep.len() > 100 && creep.values().all(|&v| v < 13));
        // The band above the console (tile rows 13-14) is recorded too:
        // 16 + 12 tiles, one of them a rare 6-12 variant.
        // Plus the nine tiles around the Hatchery (one a rare 6-12 variant).
        assert_eq!(creep.len(), 132 + 28 + 9);
        assert_eq!(creep.get(&(75, 9)), Some(&9));
        assert_eq!(creep.get(&(70, 12)), Some(&5));
        assert_eq!(creep.get(&(66, 13)), Some(&5));
        assert_eq!(creep.get(&(81, 13)), Some(&9));
        assert_eq!(creep.get(&(79, 14)), Some(&0));
    }
}

#[cfg(test)]
mod trigger_portrait_tests {
    use super::latest_trigger_portrait;

    #[test]
    fn transmissions_make_the_local_players_portrait_talk_once() {
        let tx = [(0, 1, 10, 3000), (1, 2, 11, 2000)];
        let tp = [(0, 7, 1500)];
        // Player 0: its own records only, talking portraits after transmissions.
        let (hit, seen) = latest_trigger_portrait(&tx, &tp, (0, 0), 0);
        assert_eq!((hit, seen), (Some((7, 1500)), (2, 1)));
        // Already shown: nothing new.
        assert_eq!(latest_trigger_portrait(&tx, &tp, seen, 0).0, None);
        // Another player's transmission is not ours.
        assert_eq!(latest_trigger_portrait(&tx[1..], &[], (0, 0), 0).0, None);
        // A new world (shorter lists) starts over.
        let (hit, _) = latest_trigger_portrait(&tx[..1], &[], (5, 5), 0);
        assert_eq!(hit, Some((1, 3000)));
    }
}

#[cfg(test)]
mod controller_wheel_tests {
    use super::{gamepad, wheel_page_for_selection, wheel_pages_for};
    use galaxycraft::buttons::{Act, WheelCategory, card_of_set};
    use galaxycraft::ids::unit as uid;
    use galaxycraft::map::Terrain;
    use galaxycraft::sim::{Race, World};
    use std::sync::Arc;

    fn world() -> World {
        let data = Arc::new(galaxycraft::data::GameData::bundled().expect("tables"));
        let mut w = World::new(data, Terrain::open(64, 64), 2);
        w.players[0].race = Race::Terran;
        w
    }

    fn completed(w: &mut World, id: usize) {
        w.units[id].completed = true;
    }

    #[test]
    fn selection_ring_is_dynamic_and_intersects_a_group() {
        let mut w = world();
        let marine = w.spawn_px(0, uid::MARINE, 600, 600).expect("marine");
        completed(&mut w, marine);
        let scv = w.spawn_px(0, uid::SCV, 620, 600).expect("scv");
        completed(&mut w, scv);
        w.players[0].techs[galaxycraft::ids::tech::STIM_PACKS as usize] = true;

        let one = wheel_page_for_selection(&w, &[w.units[marine].clone()], None);
        assert_eq!(one.title, "Unit Commands");
        assert!(one.entries.len() <= 9);
        assert!(one.entries.iter().any(|e| e.act == Act::Move));
        assert!(one.entries.iter().any(|e| e.act == Act::StimPacks));

        let group =
            wheel_page_for_selection(&w, &[w.units[marine].clone(), w.units[scv].clone()], None);
        assert_eq!(group.title, "Group Commands");
        assert!(group.entries.len() <= 9);
        assert!(group.entries.iter().any(|e| e.act == Act::Move));
        assert!(group.entries.iter().any(|e| e.act == Act::Attack));
        assert!(
            !group.entries.iter().any(|e| e.act == Act::StimPacks),
            "Stim is Marine-only"
        );
        assert!(
            !group.entries.iter().any(|e| e.act == Act::Gather),
            "Gather is SCV-only"
        );
    }

    #[test]
    fn building_ring_uses_only_the_building_card() {
        let mut w = world();
        let cc = w.spawn_px(0, uid::COMMAND_CENTER, 600, 600).expect("cc");
        completed(&mut w, cc);
        let page = wheel_page_for_selection(&w, &[w.units[cc].clone()], None);
        assert_eq!(page.title, "Building Commands");
        assert!(page.entries.len() <= 9);
        assert!(page.entries.iter().any(|e| e.act == Act::Train));
        assert!(!page.entries.iter().any(|e| e.act == Act::Gather));
    }

    #[test]
    fn worker_wheel_uses_the_real_build_cards() {
        let mut w = world();
        let scv = w.spawn_px(0, uid::SCV, 600, 600).expect("scv");
        completed(&mut w, scv);
        let pages = wheel_pages_for(&w, &w.units[scv]);
        let titles: Vec<&str> = pages.iter().map(|p| p.title).collect();
        assert!(titles.contains(&WheelCategory::Commands.title()));
        assert!(titles.contains(&WheelCategory::Build.title()));
        assert!(titles.contains(&WheelCategory::Stances.title()));
        let build = pages
            .iter()
            .find(|p| p.title == WheelCategory::Build.title())
            .expect("build page");
        assert!(
            build.entries.iter().any(|e| e.set == Some(239)),
            "Terran basic build card"
        );
        for entry in &build.entries {
            assert!(matches!(entry.act, Act::Build | Act::DroneBuild));
            assert!(entry.slot < 9);
            let card = card_of_set(&w, &w.units[scv], entry.set);
            let (button, _) = card[entry.slot].expect("wheel slot exists on the card");
            assert_eq!(Act::of(button.act), entry.act);
        }
    }

    #[test]
    fn wheel_page_selection_resolves_to_a_card_command() {
        let mut w = world();
        let cc = w.spawn_px(0, uid::COMMAND_CENTER, 600, 600).expect("cc");
        completed(&mut w, cc);
        let pages = wheel_pages_for(&w, &w.units[cc]);
        let train_page = pages
            .iter()
            .position(|p| p.title == WheelCategory::Train.title())
            .expect("train page");
        let focus = pages[train_page]
            .entries
            .iter()
            .position(|e| e.act == Act::Train)
            .expect("SCV train button");
        // Open/close and page selection are the real WheelState paths used
        // by the controller, not a test-only shortcut.
        let mut wheel = gamepad::WheelState::default();
        assert!(wheel.open(pages.iter().map(|p| p.entries.len())));
        while wheel.page() != train_page {
            assert!(wheel.page_delta(1));
        }
        while wheel.focus() != focus {
            assert!(wheel.focus_delta(1));
        }
        let (page, sector) = wheel.activate().expect("A activates");
        assert!(!wheel.is_open(), "activation closes the wheel");
        let entry = &pages[page].entries[sector];
        assert_eq!(entry.act, Act::Train);
        assert_eq!(entry.set, None, "the Command Center's own card");
        let card = card_of_set(&w, &w.units[cc], entry.set);
        let (button, _) = card[entry.slot].expect("train slot");
        assert_eq!(Act::of(button.act), Act::Train);
        assert_eq!(button.act_var, uid::SCV);
    }

    #[test]
    fn empty_or_pending_selections_do_not_build_a_wheel() {
        let mut w = world();
        let scv = w.spawn_px(0, uid::SCV, 600, 600).expect("scv");
        completed(&mut w, scv);
        let pages = wheel_pages_for(&w, &w.units[scv]);
        assert!(!pages.is_empty());
        // Every entry points at a real visible card slot; Card navigation
        // buttons are represented by pages, never by a wheel sector.
        for page in pages {
            assert!(!page.entries.is_empty(), "empty pages are dropped");
            for entry in page.entries {
                assert_ne!(entry.act, Act::Card);
            }
        }
    }
}

#[cfg(test)]
mod mission_scrollbar_tests {
    use super::{Framebuffer, draw_mission_scrollbar};

    /// Live capture 21 (list at the top): thumb rows 519..560 are #ff0000,
    /// the track at y 700 is #6b0000 at x 1771, the arrow apex is red.
    #[test]
    fn scrollbar_matches_the_live_capture_geometry() {
        let mut fb = Framebuffer::new(2560, 1440);
        draw_mission_scrollbar(&mut fb, 0, 1.0, 0.0);
        let red = |x: i32, y: i32| fb.get(x, y).0 >> 16 & 0xff;
        assert!(red(1790, 521) > 240, "thumb top stroke");
        assert!(red(1794, 540) < 20, "thumb interior is hollow");
        assert!(
            red(1771, 700) > 90 && red(1771, 700) < 120,
            "dark track box"
        );
        assert!(red(1794, 465) > 200, "up arrow apex");
        assert!(red(1794, 925) > 200, "down arrow apex");
        // Last page: the thumb moves to 834..876 and leaves the top empty.
        let mut end = Framebuffer::new(2560, 1440);
        draw_mission_scrollbar(&mut end, 0, 1.0, 1.0);
        assert!((end.get(1790, 836).0 >> 16 & 0xff) > 240);
        assert!((end.get(1790, 521).0 >> 16 & 0xff) < 20);
    }
}

#[cfg(test)]
mod retail_timing_tests {
    use super::{cursor_frame_index, cursor_scale_for_height, menu_clip_timeline};
    use std::time::Duration;

    #[test]
    fn cursor_cycles_five_frames_at_six_fps() {
        // 1/6 s per frame: ten 60 Hz samples.
        assert_eq!(cursor_frame_index(Duration::ZERO, 5), 0);
        assert_eq!(cursor_frame_index(Duration::from_millis(166), 5), 0);
        assert_eq!(cursor_frame_index(Duration::from_millis(167), 5), 1);
        assert_eq!(cursor_frame_index(Duration::from_millis(333), 5), 1);
        assert_eq!(cursor_frame_index(Duration::from_millis(334), 5), 2);
        assert_eq!(cursor_frame_index(Duration::from_millis(500), 5), 3);
        assert_eq!(cursor_frame_index(Duration::from_millis(667), 5), 4);
        assert_eq!(cursor_frame_index(Duration::from_millis(833), 5), 4);
        assert_eq!(cursor_frame_index(Duration::from_millis(834), 5), 0);
        // Single-frame sources never index out of bounds.
        assert_eq!(cursor_frame_index(Duration::from_secs(10), 1), 0);
        assert_eq!(cursor_frame_index(Duration::from_secs(10), 0), 0);
    }

    #[test]
    fn cursor_is_hd_art_scaled_to_screen_height() {
        // Measured live: 56-57 px tall at 1440p and 42-43 px at 1080p.
        assert!((cursor_scale_for_height(1440) - 0.75).abs() < 1e-6);
        assert!((cursor_scale_for_height(1080) - 0.5625).abs() < 1e-6);
        assert!((cursor_scale_for_height(1920) - 1.0).abs() < 1e-6);
        // A tiny framebuffer still draws at least a pixel-scale arrow.
        assert!(
            (cursor_scale_for_height(1) - 0.25).abs() < 1e-6,
            "retail floor"
        );
    }

    #[test]
    fn campaign_clips_use_the_same_retail_10_5_fps_timeline() {
        // Authored at 15 fps: the three disks are 36 frames (2.4 s), every
        // portrait 76 (5.067 s) and the On overlays 20/21/31.  The retail
        // holds each authored frame for 1/10.5 s, exactly like the menu.
        let (_, frames, duration) = menu_clip_timeline(2.4, 0.0);
        assert_eq!(frames, 36);
        assert!((duration - 36.0 / 10.5).abs() < 0.001, "{duration}");
        let (_, frames, duration) = menu_clip_timeline(76.0 / 15.0, 0.0);
        assert_eq!(frames, 76);
        assert!((duration - 76.0 / 10.5).abs() < 0.001, "{duration}");
        for frames_authored in [20usize, 21, 31] {
            let (_, frames, _) = menu_clip_timeline(frames_authored as f32 / 15.0, 0.0);
            assert_eq!(frames, frames_authored);
        }
    }

    #[test]
    fn menu_clips_pause_off_screen_and_never_fast_forward() {
        use super::{clip_lag_shift, clip_pause_shift};
        // 60 Hz draws keep the clock; a 3 s visit to another screen is
        // skipped instead of being played back at high speed.
        assert_eq!(clip_pause_shift(Duration::from_millis(16)), Duration::ZERO);
        assert_eq!(
            clip_pause_shift(Duration::from_secs(3)),
            Duration::from_secs(3)
        );
        // At 10.5 fps, 30 frames ahead of the decoder: slide 29 frames.
        let shift = clip_lag_shift(31, 1, 10.5).expect("re-anchored");
        assert!((shift.as_secs_f32() - 29.0 / 10.5).abs() < 1e-3);
        assert_eq!(clip_lag_shift(3, 1, 10.5), None, "two frames of slack");
    }

    #[test]
    fn menu_clip_order_matches_the_retail_overlaps() {
        use super::MENU_CLIP_ORDER;
        let pos = |c: (usize, bool)| MENU_CLIP_ORDER.iter().position(|&o| o == c).unwrap();
        // 130704: the Single Player fleet covers the SingleOn grid.
        assert!(pos((0, true)) < pos((0, false)));
        // 130712 / 130926: MultiOn and ExitOn cover their idle clips.
        assert!(pos((1, true)) > pos((1, false)));
        assert!(pos((7, true)) > pos((7, false)));
        assert_eq!(MENU_CLIP_ORDER.len(), 6);
    }

    #[test]
    fn menu_layers_use_the_measured_retail_10_5_fps_loop() {
        // Live capture: 151-frame Single/Multi loop every 14.4 s; the
        // 105-frame Exit clip loops every 10.0 s.
        let (start, frames, duration) = menu_clip_timeline(10.067, 0.0);
        assert_eq!(start, 0.0);
        assert_eq!(frames, 151);
        assert!((duration - 14.381).abs() < 0.01, "{duration}");
        let (_, frames, duration) = menu_clip_timeline(7.0, 0.0);
        assert_eq!(frames, 105);
        assert!((duration - 10.0).abs() < 0.01, "{duration}");
        // A phase override still lands on an authored frame.
        let (start, frames, duration) = menu_clip_timeline(10.067, 2.5);
        assert!((start - 2.5333).abs() < 0.001, "{start}");
        assert_eq!(frames, 113);
        assert!((duration - 10.762).abs() < 0.01, "{duration}");
    }
}

#[cfg(test)]
mod options_chrome_tests {
    //! Retail Options chrome (130933..130954) at the 2560x1440 content box.
    use super::{
        CSS_INNER, CSS_OUTER, CefSpace, Framebuffer, Rgb, draw_options_plate, opt_checkbox,
        opt_dropdown,
    };

    const ROWS: [f32; 5] = [345.0, 438.0, 531.0, 624.0, 717.0];
    const BG: Rgb = Rgb::new(100, 100, 100);

    fn plate(active: usize) -> Framebuffer {
        let mut fb = Framebuffer::new(2560, 1440);
        fb.clear(BG);
        let space = CefSpace {
            cx: 0.0,
            kx: 1.0,
            ky: 1.0,
        };
        draw_options_plate(&mut fb, space, &ROWS, active);
        fb
    }

    #[test]
    fn plate_is_l_shaped_below_the_tab_column() {
        let fb = plate(0);
        // Left border of the tab boxes (x 513.1..517.7) stops at y 820.5;
        // below it the dimmed menu shows through (130954: planet + labels).
        assert_eq!(fb.get(515, 500), CSS_OUTER);
        assert_eq!(fb.get(515, 900), BG, "no plate left of the content column");
        assert_eq!(fb.get(700, 1149), BG, "bottom band starts at x 837");
        assert_eq!(fb.get(1000, 1149), CSS_OUTER);
        // The content column keeps its own divider down to the bottom.
        assert_eq!(fb.get(839, 1000), CSS_OUTER);
        assert_eq!(fb.get(846, 1000), CSS_INNER);
    }

    #[test]
    fn gaps_between_tab_boxes_are_left_unfilled() {
        let fb = plate(0);
        // 355.5..361.5 separates the title strip from the Speed box.
        assert_eq!(fb.get(700, 358), BG, "gap must not be filled");
        assert_ne!(fb.get(700, 300), BG, "title strip is filled");
        assert_ne!(fb.get(700, 400), BG, "tab box is filled");
    }

    #[test]
    fn selected_tab_opens_both_divider_bands() {
        // Network (row 717) selected: the divider breaks at y ~780.
        let open = plate(4);
        assert_ne!(open.get(839, 780), CSS_OUTER);
        assert_ne!(open.get(846, 780), CSS_INNER);
        let closed = plate(0);
        assert_eq!(closed.get(839, 780), CSS_OUTER);
        assert_eq!(closed.get(846, 780), CSS_INNER);
    }

    #[test]
    fn disabled_checked_checkbox_is_grey() {
        // 130933 "Use Mouse Confine": #404040 ring and a #808080 mark.
        let mut fb = Framebuffer::new(64, 64);
        fb.clear(Rgb::new(0, 0, 0));
        opt_checkbox(&mut fb, &mut None, 10, 10, 29, true, false, "", false, 24.0);
        assert_eq!(fb.get(25, 25), Rgb::new(128, 128, 128), "grey mark");
        assert_eq!(fb.get(11, 25), Rgb::new(64, 64, 64), "grey ring");
        let mut on = Framebuffer::new(64, 64);
        on.clear(Rgb::new(0, 0, 0));
        opt_checkbox(&mut on, &mut None, 10, 10, 29, true, true, "", false, 24.0);
        assert_eq!(on.get(25, 25), Rgb::new(0xfc, 0xfc, 0x4c), "yellow mark");
        assert_eq!(on.get(11, 25), CSS_INNER, "red ring");
    }

    #[test]
    fn checkbox_mark_has_a_soft_glow_edge() {
        // The CEF mark ramps over ~3.7 px (130933: 0.07/0.33/0.6/0.87/1.0).
        let mut fb = Framebuffer::new(64, 64);
        fb.clear(Rgb::new(0, 0, 0));
        opt_checkbox(&mut fb, &mut None, 10, 10, 29, true, true, "", false, 24.0);
        // Box 9.7..39.6 -> mark 50 % edge at x 17.2; ramp over 16..18.
        let g = |x: i32| (fb.get(x, 25).0 >> 8) & 0xff;
        assert!(
            g(15) < g(16) && g(16) < g(17) && g(17) < g(18) && g(18) < g(19),
            "ramp"
        );
        assert!(
            g(17) > 80 && g(17) < 200,
            "partial coverage at the edge: {}",
            g(17)
        );
    }

    #[test]
    fn enabled_dropdown_has_the_yellow_chevron_and_no_chevron_when_disabled() {
        let mut fb = Framebuffer::new(560, 80);
        fb.clear(Rgb::new(0, 0, 0));
        opt_dropdown(&mut fb, &mut None, 20, 10.0, 516, "", true, 1.0);
        // Chevron centre 25.9 px left of the right edge, near its apex row.
        let c = fb.get(20 + 516 - 26, 10 + 16 + 12);
        let (r, g, b) = ((c.0 >> 16) & 0xff, (c.0 >> 8) & 0xff, c.0 & 0xff);
        assert!(r > 150 && g > 150 && b < 40, "yellow chevron: {r},{g},{b}");
        let mut off = Framebuffer::new(560, 80);
        off.clear(Rgb::new(0, 0, 0));
        opt_dropdown(&mut off, &mut None, 20, 10.0, 516, "", false, 1.0);
        assert_eq!(off.get(20 + 516 - 26, 10 + 16 + 12), Rgb::new(0, 0, 0));
    }
}

#[cfg(test)]
mod console_anchor_tests {
    use super::{UiRect, console_dialog_shift, menu_plate_rect};

    #[test]
    fn statdata_centres_in_whole_game_px_and_the_rest_in_screen_px() {
        // 2560x1440 content, 853 game px view (3 px per game px).
        let shift = |d| console_dialog_shift(d, 853, 2560, 3.0);
        assert_eq!(shift("minimap"), 0.0);
        assert_eq!(shift("statdata"), 106.0, "(853 - 640) / 2, integer");
        assert!((shift("tips_dlg") - 106.667).abs() < 0.01);
        assert!((shift("objctdlg") - 106.667).abs() < 0.01);
        assert!((shift("statres") - 213.333).abs() < 0.01);
        // The Nexus name control (statdata -5, classic x 240) lands at
        // (240 + 106) * 3 = 1038 px, 2 px left of the fractional 1040.
        assert_eq!(((240.0 + shift("statdata")) * 3.0).round() as i32, 1038);
    }

    #[test]
    fn menu_plate_lands_where_the_retail_template_matched() {
        // stat_f10 control 1 = classic (416, 388, 64, 20) -> (1888, 1164)
        // at 1440p; the 240x96 dlg frame matched at (1885, 1164) on
        // 135020 / 135051 / 134905.
        let r = UiRect::new(1888, 1164, 192, 60);
        assert_eq!(menu_plate_rect(r, 240, 96, 1.0), (1885, 1164, 180, 72));
    }
}

#[cfg(test)]
mod glue_pane_tests {
    use super::{PaneCell, glue_pane_cell};

    /// Row extents of the pane, as the 134919 / 134931 captures measure them
    /// (x 702..=1878, y 96..=863 at 1440p).
    #[test]
    fn pane_corners_are_rounded_like_the_retail_dialog() {
        let (w, h, g) = (1177, 768, 3);
        let last = |dy: i32| {
            (0..w)
                .rev()
                .find(|&dx| glue_pane_cell(dx, dy, w, h, g) != PaneCell::Outside)
                .map(|dx| 702 + dx)
        };
        // Rows 96-98 end at 1869, 99-104 at 1875, then 1878; the bottom
        // mirrors it (855-860 at 1875, 861-863 at 1869).
        assert_eq!(last(0), Some(1869));
        assert_eq!(last(3), Some(1875));
        assert_eq!(last(9), Some(1878));
        assert_eq!(last(767 - 8), Some(1875));
        assert_eq!(last(767), Some(1869));
        // A one-game-px border, then the fill.
        assert_eq!(glue_pane_cell(0, 400, w, h, g), PaneCell::Border);
        assert_eq!(glue_pane_cell(2, 400, w, h, g), PaneCell::Border);
        assert_eq!(glue_pane_cell(3, 400, w, h, g), PaneCell::Fill);
        assert_eq!(glue_pane_cell(600, 5, w, h, g), PaneCell::Fill);
        assert_eq!(glue_pane_cell(600, 2, w, h, g), PaneCell::Border);
        assert_eq!(glue_pane_cell(1, 1, w, h, g), PaneCell::Outside);
    }
}

#[cfg(test)]
mod hunters_smoke_tests {
    use super::{Race, hunters_capture_minimap_terrain, hunters_capture_smoke, hunters_smoke};

    #[test]
    fn each_protoss_capture_shows_its_own_geyser_smoke() {
        // 135051 keeps the game's overlay 2; 135109 caught overlay 0 (at
        // the same LO offset as in the Zerg game) and 135114 / 135121 caught
        // the geyser between puffs.
        assert_eq!(hunters_capture_smoke("ingame-protoss"), None);
        assert_eq!(hunters_smoke(Race::Protoss).map(|s| s.0), Some(432));
        let shields = hunters_capture_smoke("ingame-protoss-tip-shields").flatten();
        assert_eq!(shields.map(|s| (s.0, s.1)), Some((430, 3)));
        assert_eq!(
            shields.map(|s| (s.2, s.3)),
            hunters_smoke(Race::Zerg).map(|s| (s.2, s.3)),
            "overlay 0 has one LO offset"
        );
        for shot in [
            "ingame-protoss-tip-probe",
            "ingame-protoss-tip-minimap",
            "objectives",
            "tips",
        ] {
            assert_eq!(hunters_capture_smoke(shot), Some(None), "{shot}");
        }
        // 135121's tooltip reads "Hide Terrain" (810): its terrain is shown.
        assert_eq!(super::minimap_toggle_string(true), 810);
        assert_eq!(super::minimap_toggle_string(false), 809);
        // Only the first Zerg capture shows the terrain minimap.
        assert!(hunters_capture_minimap_terrain("ingame-zerg"));
        assert!(hunters_capture_minimap_terrain(
            "ingame-protoss-tip-minimap"
        ));
        for shot in ["objectives", "tips", "ingame-protoss", "ingame-terran2"] {
            assert!(!hunters_capture_minimap_terrain(shot), "{shot}");
        }
    }
}

#[cfg(test)]
mod dialog_button_tests {
    use super::button_label_base;

    #[test]
    fn defpushbutton_labels_sit_six_px_lower() {
        // 134931 `OK` (DEFPUSHBUTTON) vs `Previous Tip` (PUSHBUTTON): ink
        // tops 769 vs 763 on the same button row.
        assert_eq!(
            button_label_base("DEFPUSHBUTTON") - button_label_base("PUSHBUTTON"),
            6.0
        );
        // Measured against the old shared 16: PUSHBUTTON 8 px higher.
        assert_eq!(button_label_base("PUSHBUTTON"), 8.0);
    }
}

#[cfg(test)]
mod chat_tests {
    use super::{Key, chat_char};

    #[test]
    fn chat_typing_takes_letters_and_digits_in_lower_case() {
        assert_eq!(chat_char(Key::A), Some('a'));
        assert_eq!(chat_char(Key::Z), Some('z'));
        assert_eq!(chat_char(Key::Key0), Some('0'));
        assert_eq!(chat_char(Key::Key9), Some('9'));
        // The line's controls are not text.
        assert_eq!(chat_char(Key::Space), None);
        assert_eq!(chat_char(Key::Enter), None);
        assert_eq!(chat_char(Key::Backspace), None);
    }
}

#[cfg(test)]
mod card_shift_tests {
    use super::{Race, card_draw_shift};

    #[test]
    fn zerg_and_protoss_cards_draw_one_game_px_left() {
        assert_eq!(card_draw_shift(Race::Terran), (0.0, 0.0));
        assert_eq!(card_draw_shift(Race::Zerg), (-3.0, 1.0));
        assert_eq!(card_draw_shift(Race::Protoss), (-3.0, 1.0));
    }
}

#[cfg(test)]
mod hud_font_tests {
    use super::{HUD_FONT, HUD_NAME_PX, HUD_SMALL_PX};

    #[test]
    fn hud_text_matches_the_retail_ink_widths() {
        let root = std::env::var("GC_RETAIL_CACHE").unwrap_or_else(|_| {
            format!(
                "{}/retail-864772b9",
                galaxycraft::retail::user_cache_root().display()
            )
        });
        let Some(font) =
            galaxycraft::text::UiFont::load(&std::path::Path::new(&root).join(HUD_FONT))
        else {
            eprintln!("skip: retail fonts not materialised on this machine");
            return;
        };
        // Retail ink widths (lavender ink, 135051 / 134931): the stat row,
        // the unit name and the tips checkbox label.
        for (text, px, retail) in [
            ("Psi Used: 4", HUD_SMALL_PX, 128.0),
            ("Psi Max: 200", HUD_SMALL_PX, 147.0),
            ("Protoss Nexus", HUD_NAME_PX, 204.0),
            ("Show Tips at Startup", HUD_NAME_PX, 305.0),
        ] {
            let (x0, x1, _, _) = font.ink_bounds_cef(text, px, 0.0).expect("ink");
            let w = x1 - x0;
            assert!(
                (w - retail).abs() <= 2.0,
                "{text}: {w:.1} px vs retail {retail}"
            );
        }
    }
}

#[cfg(test)]
mod main_menu_hover_tests {
    use super::{
        MENU_CLIP_ORDER, Rgb, label_drop_shadow, main_label_hover_colors, menu_clip_visibility,
    };

    #[test]
    fn hidden_mouse_over_clips_keep_running_and_only_show_on_hover() {
        // Retail screencast 2026-11-05 20:41:38: SingleOn never pauses or
        // restarts; it is only hidden.  Every clip is pulled every frame.
        for (idx, is_hover) in MENU_CLIP_ORDER {
            for hover in [None, Some(0), Some(1), Some(7)] {
                let v = menu_clip_visibility(idx, is_hover, hover);
                assert_eq!(
                    v.drawn,
                    !is_hover || hover == Some(idx),
                    "{idx} {is_hover} {hover:?}"
                );
            }
        }
        // The idle layers stay drawn under every hover.
        assert!(menu_clip_visibility(0, false, Some(0)).drawn);
        assert!(!menu_clip_visibility(0, true, Some(1)).drawn);
    }

    #[test]
    fn hover_only_recolours_main_menu_labels() {
        let (normal, bright, hover) = (
            Rgb::new(102, 227, 53),
            Rgb::new(208, 255, 131),
            Rgb::new(196, 255, 124),
        );
        assert_eq!(
            main_label_hover_colors(false, normal, bright, hover),
            (normal, bright)
        );
        assert_eq!(
            main_label_hover_colors(true, normal, bright, hover),
            (hover, bright)
        );
    }

    #[test]
    fn labels_share_the_footer_drop_shadow() {
        // One UI pixel down-right in opaque black: 3 px at 1440p (measured on
        // reference 130637), 2 px at 960p, never less than 1 px.
        assert_eq!(label_drop_shadow(3.0), (3, Rgb::new(0, 0, 0)));
        assert_eq!(label_drop_shadow(2.0).0, 2);
        assert_eq!(label_drop_shadow(0.4).0, 1);
    }
}

#[cfg(test)]
mod letterbox_tests {
    use super::{Framebuffer, VIEW_H, VIEW_W, letterbox, present_into, surface_point};

    #[test]
    fn sixteen_by_nine_and_ultrawide_draw_at_the_window_size() {
        for (w, h) in [
            (1280, 720),
            (1920, 1080),
            (2560, 1440),
            (3440, 1440),
            (2560, 1080),
        ] {
            assert_eq!(letterbox(w, h), (w, h, 0, 0), "{w}x{h}");
        }
    }

    #[test]
    fn narrow_windows_letterbox_a_sixteen_by_nine_surface() {
        // The reported window (2026-11-05): 1277x954.
        assert_eq!(letterbox(1277, 954), (1277, 718, 0, 118));
        assert_eq!(letterbox(1280, 960), (1280, 720, 0, 120));
        assert_eq!(letterbox(1920, 1200), (1920, 1080, 0, 60));
        for (w, h) in [(1277, 954), (1024, 768), (1920, 1200), (1280, 1024)] {
            let (iw, ih, ox, oy) = letterbox(w, h);
            assert_eq!(iw, w);
            assert!((iw as f32 / ih as f32 - 16.0 / 9.0).abs() < 0.01, "{w}x{h}");
            assert_eq!(ox, 0);
            assert!(oy >= 0 && oy as usize + ih <= h);
        }
    }

    #[test]
    fn the_default_window_is_720p() {
        assert_eq!((VIEW_W, VIEW_H), (1280, 720));
        assert_eq!(letterbox(VIEW_W, VIEW_H).3, 0);
    }

    #[test]
    fn presenting_adds_black_bars_and_the_pointer_follows_the_surface() {
        let mut inner = Framebuffer::new(4, 2);
        inner.pixels.fill(0x00ff_ffff);
        let mut out = Vec::new();
        present_into(&inner, 4, 6, (0, 2), &mut out);
        assert_eq!(out.len(), 24);
        assert!(out[..8].iter().all(|&p| p == 0), "top bar");
        assert!(out[8..16].iter().all(|&p| p == 0x00ff_ffff), "surface");
        assert!(out[16..].iter().all(|&p| p == 0), "bottom bar");
        // A click on the first surface row lands on surface row 0.
        assert_eq!(surface_point((0, 2), 3, 2), (3, 0));
        assert_eq!(surface_point((0, 118), 640, 477), (640, 359));
    }
}

#[cfg(test)]
mod construction_look_tests {
    use super::{ConstructionStage, terran_construction_stage};
    use galaxycraft::ids::unit as uid;

    #[test]
    fn scaffold_frames_switch_at_the_recorded_progress_marks() {
        // Boot Camp recording: a 600-frame depot switches at 1/9, 1/3, 5/9.
        let data = std::sync::Arc::new(galaxycraft::data::GameData::bundled().expect("tables"));
        let mut w =
            galaxycraft::sim::World::new(data.clone(), galaxycraft::map::Terrain::open(32, 32), 2);
        let i = w.spawn_px(0, uid::SUPPLY_DEPOT, 300, 300).expect("depot");
        let ty = data.unit(uid::SUPPLY_DEPOT).clone();
        assert_eq!(ty.construction_image, 330, "units.dat: TBldSml");
        let u = &mut w.units[i];
        u.completed = false;
        u.build_total = 600;
        let at = |u: &mut galaxycraft::sim::SimUnit, done: i32| {
            u.build_remaining = 600 - done;
            terran_construction_stage(u, &ty)
        };
        assert_eq!(at(u, 0), Some(ConstructionStage::Scaffold(0)));
        assert_eq!(at(u, 66), Some(ConstructionStage::Scaffold(0)));
        assert_eq!(at(u, 67), Some(ConstructionStage::Scaffold(1)));
        assert_eq!(at(u, 200), Some(ConstructionStage::Scaffold(2)));
        assert_eq!(at(u, 333), Some(ConstructionStage::Scaffold(2)));
        assert_eq!(at(u, 334), Some(ConstructionStage::AlmostBuilt));
        u.completed = true;
        assert_eq!(terran_construction_stage(u, &ty), None);
        // Not Terran: no scaffold, even unfinished.
        w.units[i].completed = false;
        let pool = data.unit(uid::SPAWNING_POOL).clone();
        assert_eq!(terran_construction_stage(&w.units[i], &pool), None);
    }
}

#[cfg(test)]
mod end_check_tests {
    use super::{END_CHECK_PERIOD, frames_to_end_check};

    #[test]
    fn the_end_dialog_waits_for_the_46_frame_game_end_check() {
        assert_eq!(END_CHECK_PERIOD, 46);
        assert_eq!(frames_to_end_check(46), 0);
        assert_eq!(frames_to_end_check(47), 45);
        assert_eq!(frames_to_end_check(91), 1);
        for t in 1..500u64 {
            let h = frames_to_end_check(t) as u64;
            assert!(h < 46 && (t + h).is_multiple_of(46));
        }
    }
}

// `impl World` split by subsystem (scripts/split_impl.py).
#[path = "sc_view/briefing.rs"]
mod briefing;
#[path = "sc_view/collections.rs"]
mod collections;
#[path = "sc_view/commands.rs"]
mod commands;
#[path = "sc_view/cursor.rs"]
mod cursor;
use cursor::*;
#[path = "sc_view/draw_world.rs"]
mod draw_world;
#[path = "sc_view/fog_memory.rs"]
mod fog_memory;
#[path = "sc_view/game_dialog.rs"]
mod game_dialog;
#[path = "sc_view/game_input.rs"]
mod game_input;
#[path = "sc_view/game_loop.rs"]
mod game_loop;
#[path = "sc_view/hotkeys.rs"]
mod hotkeys;
#[path = "sc_view/hud.rs"]
mod hud;
#[path = "sc_view/intro.rs"]
mod intro;
#[path = "sc_view/lobby.rs"]
mod lobby;
#[path = "sc_view/menu_draw.rs"]
mod menu_draw;
#[path = "sc_view/menu_input.rs"]
mod menu_input;
#[path = "sc_view/net.rs"]
mod net;
#[path = "sc_view/options.rs"]
mod options;
#[path = "sc_view/portrait_panel.rs"]
mod portrait_panel;
#[path = "sc_view/reference.rs"]
mod reference;
#[path = "sc_view/results.rs"]
mod results;
#[path = "sc_view/session.rs"]
mod session;
#[path = "sc_view/sounds.rs"]
mod sounds;
#[path = "sc_view/turrets.rs"]
mod turrets;
