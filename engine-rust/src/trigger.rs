//! CHK trigger parsing and a deterministic subset of the trigger VM.
//!
//! A `TRIG` record is 2,400 bytes: 16 conditions (20 bytes each), 64 actions
//! (32 bytes each), a 32-bit execution-flags field and 28 per-player/force
//! enabled bytes.  The layout is the one used by the shipped `triggers\*.trg`
//! files and by the campaign maps.
//!
//! The classic condition/action ids are implemented.  StarCraft Remastered
//! adds a few extended ids; unknown ids are ignored rather than guessed, and
//! that limitation is recorded in `docs/EVIDENCE.md`.

use crate::sim::{Order, World};

pub const RECORD_SIZE: usize = 2400;
pub const TRIGGER_INTERVAL: u64 = 30;
/// Ticks per trigger "game second" (Countdown Timer, Elapsed Time).  FACT
/// (Desperate Alliance retail recording): the countdown loses 446-447
/// seconds every 300 real seconds at Fastest (23.81 ticks/s), one every
/// 16.0 ticks; Wasteland's "Elapsed Time at least 8" SCV line then lands
/// on the tick-150 cycle, 6.25 s in (retail 6.5 s; the old 24-tick second
/// gave 8.75 s).
pub const GAME_SECOND_TICKS: u64 = 16;

/// `(player, four-byte script id, optional centre)` queued by trigger action
/// 15/16.
pub type PendingAiScript = (usize, [u8; 4], Option<(i32, i32)>);

#[derive(Clone, Debug, Default)]
pub struct TriggerCondition {
    /// 1-based index into the scenario's `MRGN` locations (0 = anywhere).
    pub location: u32,
    /// Player/force selector.
    pub group: u32,
    pub quantity: u32,
    pub unit_id: u16,
    /// 0 = at least, 1 = at most, 2 = exactly.
    pub comparison: u8,
    pub kind: u8,
    pub amount_modifier: u8,
}

/// How long a Transmission talks (ms).  INFERENCE from the public CHK
/// trigger spec (staredit.net `TRIG`): `time` holds the WAV length, the
/// second group field the modifier amount and `num` the modifier (7 set to,
/// 8 add, 9 subtract).  Falsifier: a retail transmission whose portrait
/// stops talking at a different moment.
pub fn transmission_ms(action: &TriggerAction) -> u32 {
    match action.num {
        7 => action.group2,
        8 => action.time.saturating_add(action.group2),
        9 => action.time.saturating_sub(action.group2),
        _ => action.time,
    }
}

/// Action flag "Always Display": the text shows even with subtitles off.
/// INFERENCE from the public CHK spec (staredit.net `TRIG`, action flags
/// bit 2); falsifier: a retail map whose flagged Display Text hides with
/// subtitles off.
pub const ALWAYS_DISPLAY: u8 = 0x04;

/// One thing a trigger shows or plays (`string` = 1-based `STR` id, 0 =
/// none).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriggerCue {
    Text {
        player: usize,
        string: u32,
        always: bool,
        /// Transmission length (0 for Display Text Message).
        ms: u32,
    },
    Wav {
        player: usize,
        string: u32,
    },
}

impl TriggerCue {
    pub fn player(&self) -> usize {
        match *self {
            TriggerCue::Text { player, .. } | TriggerCue::Wav { player, .. } => player,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct TriggerAction {
    pub location: u32,
    pub text_id: u32,
    pub sound_id: u32,
    pub time: u32,
    pub group: u32,
    pub group2: u32,
    pub extra: u16,
    pub kind: u8,
    pub num: u8,
    pub flags: u8,
}

#[derive(Clone, Debug, Default)]
pub struct Trigger {
    pub conditions: Vec<TriggerCondition>,
    pub actions: Vec<TriggerAction>,
    pub execution_flags: u32,
    pub enabled: [bool; 28],
}

#[derive(Clone, Debug, Default)]
pub struct TriggerState {
    /// One entry per player, per trigger: has it fired and is it preserved?
    fired: Vec<Vec<bool>>,
    wait_until: Vec<u64>,
    /// Action index to resume at once a `Wait` action expires: `(trigger,
    /// next action)`.  StarCraft resumes the same trigger after a wait rather
    /// than restarting it, so this must survive the 30-tick trigger cadence.
    resume: Vec<Option<(usize, usize)>>,
    /// `(player, trigger)` that executed action 5 (Pause Game).  Only that
    /// trigger may resume its pending Wait/Transmission while paused, and
    /// finishing it lifts the pause.  Action 6 clears it early.
    pause_source: Option<(usize, usize)>,
}

/// One leaderboard request (actions 17..=21 and 33..=37).  The headless
/// engine has no UI, but the score/unit/resource query the retail game makes
/// is deterministic and is recorded here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Leaderboard {
    pub player: usize,
    pub text_id: u32,
    pub kind: u8,
    pub unit_id: u16,
    pub score_kind: u16,
    pub goal: Option<u32>,
    pub location: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct TriggerSystem {
    pub triggers: Vec<Trigger>,
    pub locations: Vec<(i32, i32, i32, i32)>,
    /// The scenario's `UPRP` slots, copied here so actions 11/44 can apply
    /// them without threading the whole scenario through the VM.
    pub unit_properties: Vec<crate::map::UnitProperties>,
    pub state: TriggerState,
    pub switches: [bool; 256],
    pub countdown_timer: u32,
    /// A Set Countdown Timer action has run: the client shows the timer
    /// from then on, also at 0:00 (retail Desperate Alliance).
    pub countdown_shown: bool,
    /// Trigger actions that start or relocate an AI script.  The match runner
    /// consumes these after the trigger VM runs.
    pub pending_ai_scripts: Vec<PendingAiScript>,
    pub paused: bool,
    /// True while action 54 (Pause Timer) is in force.
    pub timer_paused: bool,
    /// Per-player, per-trigger preserve flag set by action 3.
    pub preserved: Vec<Vec<bool>>,
    /// Presentation/state side effects that a renderer or campaign shell would
    /// consume.  They are recorded so the headless VM still executes the
    /// action instead of silently dropping it.
    pub mission_objectives: Vec<Option<u32>>,
    /// `(player, unit type, text id, duration ms)` of each Transmission
    /// (see [`transmission_ms`]).
    pub transmissions: Vec<(usize, u16, u32, u32)>,
    /// What players read and hear, in execution order: Transmission (7),
    /// Play WAV (8) and Display Text Message (9).  Presentation only (not
    /// part of `state_hash`); the client shows / plays the new entries.
    pub cues: Vec<TriggerCue>,
    pub center_views: Vec<(usize, u32)>,
    pub minimap_pings: Vec<(usize, u32)>,
    /// `(player, location)` of each Transmission: Space centres the view on
    /// the last one ("Space Bar - Last Transmission", `help_txt`).
    /// Presentation only, like `cues`.
    pub transmission_locations: Vec<(usize, u32)>,
    pub talking_portraits: Vec<(usize, u16, u32)>,
    pub muted_speech: bool,
    pub next_scenario: Option<u32>,
    pub leaderboard: Option<Leaderboard>,
    pub leaderboard_computer_players: bool,
    /// Force (0..=3) of players 1..8, from the scenario's `FORC`.
    pub forces: [u8; 8],
    /// Players 1..8 in the game (`OWNR` not 0): only they run the
    /// triggers enabled for a whole force.
    pub present: [bool; 8],
}

impl Default for TriggerSystem {
    fn default() -> Self {
        Self {
            triggers: Vec::new(),
            locations: Vec::new(),
            unit_properties: Vec::new(),
            state: TriggerState::default(),
            switches: [false; 256],
            countdown_timer: 0,
            countdown_shown: false,
            pending_ai_scripts: Vec::new(),
            paused: false,
            timer_paused: false,
            preserved: Vec::new(),
            mission_objectives: Vec::new(),
            transmissions: Vec::new(),
            cues: Vec::new(),
            center_views: Vec::new(),
            minimap_pings: Vec::new(),
            transmission_locations: Vec::new(),
            talking_portraits: Vec::new(),
            muted_speech: false,
            next_scenario: None,
            leaderboard: None,
            leaderboard_computer_players: true,
            forces: [0; 8],
            present: [true; 8],
        }
    }
}

impl TriggerSystem {
    pub fn parse(chunk: &[u8], locations: &[(i32, i32, i32, i32)]) -> Self {
        let mut triggers = Vec::new();
        for record in chunk.as_chunks::<RECORD_SIZE>().0 {
            let mut conditions = Vec::new();
            for c in 0..16 {
                let o = c * 20;
                let kind = record[o + 15];
                if kind == 0 {
                    continue;
                }
                conditions.push(TriggerCondition {
                    location: u32::from_le_bytes([
                        record[o],
                        record[o + 1],
                        record[o + 2],
                        record[o + 3],
                    ]),
                    group: u32::from_le_bytes([
                        record[o + 4],
                        record[o + 5],
                        record[o + 6],
                        record[o + 7],
                    ]),
                    quantity: u32::from_le_bytes([
                        record[o + 8],
                        record[o + 9],
                        record[o + 10],
                        record[o + 11],
                    ]),
                    unit_id: u16::from_le_bytes([record[o + 12], record[o + 13]]),
                    comparison: record[o + 14],
                    kind,
                    amount_modifier: record[o + 16],
                });
            }
            let mut actions = Vec::new();
            for a in 0..64 {
                let o = 320 + a * 32;
                let kind = record[o + 26];
                if kind == 0 {
                    continue;
                }
                actions.push(TriggerAction {
                    location: u32::from_le_bytes([
                        record[o],
                        record[o + 1],
                        record[o + 2],
                        record[o + 3],
                    ]),
                    text_id: u32::from_le_bytes([
                        record[o + 4],
                        record[o + 5],
                        record[o + 6],
                        record[o + 7],
                    ]),
                    sound_id: u32::from_le_bytes([
                        record[o + 8],
                        record[o + 9],
                        record[o + 10],
                        record[o + 11],
                    ]),
                    time: u32::from_le_bytes([
                        record[o + 12],
                        record[o + 13],
                        record[o + 14],
                        record[o + 15],
                    ]),
                    group: u32::from_le_bytes([
                        record[o + 16],
                        record[o + 17],
                        record[o + 18],
                        record[o + 19],
                    ]),
                    group2: u32::from_le_bytes([
                        record[o + 20],
                        record[o + 21],
                        record[o + 22],
                        record[o + 23],
                    ]),
                    extra: u16::from_le_bytes([record[o + 24], record[o + 25]]),
                    kind,
                    num: record[o + 27],
                    flags: record[o + 28],
                });
            }
            let mut enabled = [false; 28];
            for (i, slot) in enabled.iter_mut().enumerate() {
                *slot = record[2372 + i] != 0;
            }
            triggers.push(Trigger {
                conditions,
                actions,
                execution_flags: u32::from_le_bytes([
                    record[2368],
                    record[2369],
                    record[2370],
                    record[2371],
                ]),
                enabled,
            });
        }
        Self {
            triggers,
            locations: locations.to_vec(),
            unit_properties: Vec::new(),
            state: TriggerState::default(),
            switches: [false; 256],
            countdown_timer: 0,
            countdown_shown: false,
            pending_ai_scripts: Vec::new(),
            paused: false,
            timer_paused: false,
            preserved: Vec::new(),
            mission_objectives: Vec::new(),
            transmissions: Vec::new(),
            cues: Vec::new(),
            center_views: Vec::new(),
            minimap_pings: Vec::new(),
            transmission_locations: Vec::new(),
            talking_portraits: Vec::new(),
            muted_speech: false,
            next_scenario: None,
            leaderboard: None,
            leaderboard_computer_players: true,
            forces: [0; 8],
            present: [true; 8],
        }
    }

    pub fn process(&mut self, world: &mut World) {
        // The countdown runs on game seconds, independently of the trigger
        // cadence, unless action 54 (Pause Timer) or action 5 (Pause Game)
        // is in force.
        if world.tick > 0
            && world.tick.is_multiple_of(GAME_SECOND_TICKS)
            && !self.timer_paused
            && !self.paused
        {
            self.countdown_timer = self.countdown_timer.saturating_sub(1);
        }
        // A pause belongs to the trigger that issued it.  If that player
        // leaves the game the trigger can never finish, so lift the pause.
        if let Some((owner, _)) = self.state.pause_source
            && world.players.get(owner).is_none_or(|p| p.defeated)
        {
            self.paused = false;
            self.state.pause_source = None;
        }
        // Triggers first run at game start (elapsed time 0: `terran04`
        // sets its objectives on "Elapsed Time at most 0"), then on the
        // 30-tick cadence.
        // Conditions are evaluated on the 30-tick cadence, but a running
        // Wait / Transmission resumes on the tick it ends (retail
        // Wasteland: the 3956 ms "Howdy" line plus a 2000 ms Wait put
        // "NEW MISSION OBJECTIVE" exactly 6.0 s after it).
        let cycle = world.tick == 1 || world.tick.is_multiple_of(TRIGGER_INTERVAL);
        if !cycle {
            if self.state.fired.len() == world.players.len() {
                for player in 0..world.players.len() {
                    // While paused only the trigger that paused may resume
                    // its Wait or Transmission; every other player is held.
                    if self.paused
                        && self
                            .state
                            .pause_source
                            .is_some_and(|(owner, _)| owner != player)
                    {
                        continue;
                    }
                    if world.tick < self.state.wait_until[player] {
                        continue;
                    }
                    if let Some((ti, start)) = self.state.resume[player]
                        && (!self.paused || self.state.pause_source == Some((player, ti)))
                    {
                        self.state.resume[player] = None;
                        if let Some(trigger) = self.triggers.get(ti).cloned() {
                            self.run_actions(world, player, ti, &trigger, start);
                        }
                    }
                }
            }
            return;
        }
        if self.state.fired.len() != world.players.len() {
            self.state.fired = vec![vec![false; self.triggers.len()]; world.players.len()];
            self.state.wait_until = vec![0; world.players.len()];
            self.state.resume = vec![None; world.players.len()];
            self.preserved = vec![vec![false; self.triggers.len()]; world.players.len()];
            self.mission_objectives = vec![None; world.players.len()];
        }
        for player in 0..world.players.len() {
            // A player who is out of the game (defeated, or an empty lobby
            // slot) runs no triggers; otherwise an empty slot's melee
            // "commands at most 0 buildings" would fire for it.
            if world.players[player].defeated {
                continue;
            }
            // While paused no new trigger starts.  The player whose trigger
            // paused the game may still resume its own Wait/Transmission.
            if self.paused
                && self
                    .state
                    .pause_source
                    .is_some_and(|(owner, _)| owner != player)
            {
                continue;
            }
            if world.tick < self.state.wait_until[player] {
                continue;
            }
            // A previous Wait action resumes the rest of that trigger first.
            if let Some((ti, start)) = self.state.resume[player] {
                if !self.paused || self.state.pause_source == Some((player, ti)) {
                    self.state.resume[player] = None;
                    if let Some(trigger) = self.triggers.get(ti).cloned() {
                        self.run_actions(world, player, ti, &trigger, start);
                    }
                }
                continue;
            }
            if self.paused {
                continue;
            }
            for ti in 0..self.triggers.len() {
                let trigger = self.triggers[ti].clone();
                if !self.runs_for(&trigger, player) {
                    continue;
                }
                let preserved = self
                    .preserved
                    .get(player)
                    .and_then(|p| p.get(ti))
                    .copied()
                    .unwrap_or(false);
                if self.state.fired[player][ti] && !preserved && trigger.execution_flags & 4 == 0 {
                    continue;
                }
                if !self.conditions_hold(world, player, &trigger) {
                    continue;
                }
                self.state.fired[player][ti] = true;
                self.run_actions(world, player, ti, &trigger, 0);
                // A Wait or Transmission holds the player's later triggers
                // too (BW wait-blocking): Boot Camp's opening (Wait 1500)
                // must keep its trigger 10 ("MARINE ...") from running in
                // the same cycle, as on the retail recording.
                if self.state.resume[player].is_some() {
                    break;
                }
            }
        }
    }

    /// Execute actions from `start`, stopping at a Wait (action 4) so the same
    /// trigger can resume at the following action once the player's wait slot
    /// expires.  Preserve Trigger (action 3) is persistent state, not a
    /// one-shot action.
    fn run_actions(
        &mut self,
        world: &mut World,
        player: usize,
        trigger_index: usize,
        trigger: &Trigger,
        start: usize,
    ) {
        for (ai, action) in trigger.actions.iter().enumerate().skip(start) {
            match action.kind {
                3 => {
                    if let Some(p) = self.preserved.get_mut(player) {
                        p[trigger_index] = true;
                    }
                    continue;
                }
                4 => {
                    // Trigger Wait times are milliseconds.
                    let ticks =
                        ((action.time as u64) * crate::sim::TICKS_PER_SECOND as u64 / 1000).max(1);
                    self.state.wait_until[player] = world.tick + ticks;
                    self.state.resume[player] = Some((trigger_index, ai + 1));
                    return;
                }
                7 => {
                    // A Transmission holds its trigger for the line's
                    // length, so a conversation plays line after line
                    // (`The Trump Card` opens with three).  INFERENCE from
                    // the public CHK trigger spec; falsifier: a retail
                    // recording where the lines overlap.
                    self.execute(world, player, trigger_index, action);
                    let ms = transmission_ms(action) as u64;
                    let ticks = (ms * crate::sim::TICKS_PER_SECOND as u64 / 1000).max(1);
                    self.state.wait_until[player] = world.tick + ticks;
                    self.state.resume[player] = Some((trigger_index, ai + 1));
                    return;
                }
                _ => self.execute(world, player, trigger_index, action),
            }
        }
        // StarEdit: if the trigger that paused the game has no matching
        // Unpause Game action, the game unpauses when that trigger finishes.
        if self.state.pause_source == Some((player, trigger_index)) {
            self.paused = false;
            self.state.pause_source = None;
        }
    }

    fn conditions_hold(&self, world: &World, player: usize, trigger: &Trigger) -> bool {
        trigger
            .conditions
            .iter()
            .all(|c| self.condition_holds(world, player, c))
    }

    fn condition_holds(&self, world: &World, player: usize, c: &TriggerCondition) -> bool {
        // Condition ids are the classic CHK list (0 no condition, 1 countdown,
        // 2 command, 3 bring, ... 22 always, 23 never).
        if c.kind == 11 {
            // Switch condition: switch number is in the modifier byte, and
            // comparison 2 means "set", 3 means "cleared".
            let on = self
                .switches
                .get(c.amount_modifier as usize)
                .copied()
                .unwrap_or(false);
            return match c.comparison {
                2 => on,
                3 => !on,
                _ => false,
            };
        }
        let actual = match c.kind {
            0 | 22 => return true,
            23 => return false,
            1 => self.countdown_timer as i32,
            2 => self.command_count(world, player, c),
            3 => self.bring_count(world, player, c),
            4 => {
                let p = &world.players[player.min(world.players.len().saturating_sub(1))];
                match c.amount_modifier {
                    0 => p.minerals_gathered,
                    1 => p.gas_gathered,
                    _ => p.minerals_gathered + p.gas_gathered,
                }
            }
            5 => self.kill_count(world, player, c.unit_id),
            6 => {
                let mine = self.command_count(world, player, c);
                let most = (0..world.players.len())
                    .map(|p| self.command_count(world, p, c))
                    .max()
                    .unwrap_or(0);
                if mine == most { 1 } else { 0 }
            }
            7 => {
                let mine = self.bring_count(world, player, c);
                let most = (0..world.players.len())
                    .map(|p| self.bring_count(world, p, c))
                    .max()
                    .unwrap_or(0);
                if mine == most { 1 } else { 0 }
            }
            8 => {
                let mine = self.kill_count(world, player, c.unit_id);
                let most = (0..world.players.len())
                    .map(|p| self.kill_count(world, p, c.unit_id))
                    .max()
                    .unwrap_or(0);
                if mine == most { 1 } else { 0 }
            }
            9 => {
                let kind = c.amount_modifier as u16;
                let mine = self.score(world, player, kind);
                let most = (0..world.players.len())
                    .map(|p| self.score(world, p, kind))
                    .max()
                    .unwrap_or(0);
                if mine == most { 1 } else { 0 }
            }
            10 => {
                let mine = self.resources(world, player, c.amount_modifier);
                let most = (0..world.players.len())
                    .map(|p| self.resources(world, p, c.amount_modifier))
                    .max()
                    .unwrap_or(0);
                if mine == most { 1 } else { 0 }
            }
            11 => self
                .switches
                .get(c.quantity as usize)
                .copied()
                .unwrap_or(false) as i32,
            12 => (world.tick / GAME_SECOND_TICKS) as i32,
            13 => return false,
            14 => {
                let alive = world
                    .players
                    .iter()
                    .enumerate()
                    .filter(|(i, p)| *i != player && !p.defeated)
                    .count();
                alive as i32
            }
            15 => {
                let p = &world.players[player.min(world.players.len() - 1)];
                if c.unit_id <= 227 {
                    p.death_counts.get(c.unit_id as usize).copied().unwrap_or(0) as i32
                } else if c.unit_id == 230 || c.unit_id == 231 || c.unit_id == 232 {
                    p.death_counts
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| {
                            let building = world.data.unit(*i as u16).is_building();
                            if c.unit_id == 230 {
                                !building
                            } else {
                                building
                            }
                        })
                        .map(|(_, n)| *n)
                        .sum::<u32>() as i32
                } else {
                    p.deaths as i32
                }
            }
            16 => {
                let mine = self.command_count(world, player, c);
                let least = (0..world.players.len())
                    .map(|p| self.command_count(world, p, c))
                    .min()
                    .unwrap_or(0);
                if mine == least { 1 } else { 0 }
            }
            17 => {
                let mine = self.bring_count(world, player, c);
                let least = (0..world.players.len())
                    .map(|p| self.bring_count(world, p, c))
                    .min()
                    .unwrap_or(0);
                if mine == least { 1 } else { 0 }
            }
            18 => {
                let mine = self.kill_count(world, player, c.unit_id);
                let least = (0..world.players.len())
                    .map(|p| self.kill_count(world, p, c.unit_id))
                    .min()
                    .unwrap_or(0);
                if mine == least { 1 } else { 0 }
            }
            19 => {
                let kind = c.amount_modifier as u16;
                let mine = self.score(world, player, kind);
                let least = (0..world.players.len())
                    .map(|p| self.score(world, p, kind))
                    .min()
                    .unwrap_or(0);
                if mine == least { 1 } else { 0 }
            }
            20 => {
                let mine = self.resources(world, player, c.amount_modifier);
                let least = (0..world.players.len())
                    .map(|p| self.resources(world, p, c.amount_modifier))
                    .min()
                    .unwrap_or(0);
                if mine == least { 1 } else { 0 }
            }
            21 => self.score(world, player, c.amount_modifier as u16),
            _ => return false,
        };
        let expected = c.quantity as i32;
        // CHK comparison bytes: 0 at least, 1 at most, 10 exactly.
        match c.comparison {
            0 => actual >= expected,
            1 => actual <= expected,
            10 => actual == expected,
            _ => false,
        }
    }

    /// The `units.dat` unit classes used by trigger arguments.
    fn unit_matches(world: &World, type_id: u16, wanted: u16) -> bool {
        match wanted {
            229 => true,
            230 => !world.data.unit(type_id).is_building(),
            231 => world.data.unit(type_id).is_building(),
            232 => world.data.unit(type_id).is_building(),
            _ => type_id == wanted,
        }
    }

    /// Score category lookup (CHK score types 0..=7).
    fn score(&self, world: &World, player: usize, kind: u16) -> i32 {
        let p = &world.players[player.min(world.players.len() - 1)];
        match kind {
            0 => p.scores[1] + p.scores[2] + p.scores[4] + p.scores[5],
            3 => p.scores[1] + p.scores[2],
            6 => p.scores[4] + p.scores[5],
            1..=5 | 7 => p.scores[kind as usize],
            _ => 0,
        }
    }

    fn resources(&self, world: &World, player: usize, kind: u8) -> i32 {
        let p = &world.players[player.min(world.players.len() - 1)];
        match kind {
            0 => p.minerals,
            1 => p.gas,
            _ => p.minerals + p.gas,
        }
    }

    fn kill_count(&self, world: &World, player: usize, unit: u16) -> i32 {
        let p = &world.players[player.min(world.players.len() - 1)];
        if unit <= 227 {
            p.kill_counts.get(unit as usize).copied().unwrap_or(0) as i32
        } else if unit == 230 || unit == 231 || unit == 232 {
            p.kill_counts
                .iter()
                .enumerate()
                .filter(|(i, _)| {
                    let building = world.data.unit(*i as u16).is_building();
                    if unit == 230 { !building } else { building }
                })
                .map(|(_, n)| *n)
                .sum::<u32>() as i32
        } else {
            p.kills as i32
        }
    }

    fn command_count(&self, world: &World, player: usize, c: &TriggerCondition) -> i32 {
        let owners = self.condition_owners(world, c.group, player);
        world
            .units
            .iter()
            .filter(|u| {
                u.alive
                    && Self::unit_matches(world, u.type_id, c.unit_id)
                    && owners
                        .as_ref()
                        .is_none_or(|o| o.contains(&(u.owner as usize)))
            })
            .count() as i32
    }

    /// Players a count condition's group selects (`None` = any owner):
    /// 0..11 a player, 13 the current player, 14 Foes and 26 Non Allied
    /// Victory Players (its enemies still in the game), 15 Allies (allied
    /// players other than itself), 17 All Players, 18..21 the members of
    /// Force 1..4 in the game.
    fn condition_owners(&self, world: &World, group: u32, current: usize) -> Option<Vec<usize>> {
        let n = world.players.len();
        let cur = current as u8;
        Some(match group {
            0..=11 => vec![group as usize],
            18..=21 => self.force_members(group as usize - 18),
            13 => vec![current],
            14 | 26 => (0..n)
                .filter(|&p| world.is_enemy(cur, p as u8) && !world.players[p].defeated)
                .collect(),
            15 => (0..n)
                .filter(|&p| p != current && world.same_side(cur, p as u8))
                .collect(),
            17 => (0..n).collect(),
            _ => return None,
        })
    }

    fn bring_count(&self, world: &World, player: usize, c: &TriggerCondition) -> i32 {
        let owners = self.condition_owners(world, c.group, player);
        let loc = c.location as usize;
        world
            .units
            .iter()
            .filter(|u| {
                if !u.alive {
                    return false;
                }
                if owners
                    .as_ref()
                    .is_some_and(|o| !o.contains(&(u.owner as usize)))
                {
                    return false;
                }
                if !Self::unit_matches(world, u.type_id, c.unit_id) {
                    return false;
                }
                if loc == 0 {
                    return true;
                }
                let Some(l) = self.locations.get(loc - 1) else {
                    return false;
                };
                let x = u.x / 256;
                let y = u.y / 256;
                x >= l.0 && x < l.2 && y >= l.1 && y < l.3
            })
            .count() as i32
    }

    /// Whether `player` executes `trigger`: enabled for the player, for All
    /// Players (17) or for the player's force (18..=21, `FORC`).
    fn runs_for(&self, trigger: &Trigger, player: usize) -> bool {
        if trigger.enabled[player.min(27)] || trigger.enabled[17] {
            return true;
        }
        player < 8 && self.present[player] && trigger.enabled[18 + self.forces[player] as usize]
    }

    /// The value a leaderboard shows for `player`: units of its type under
    /// control (17/18, goal 33/34; 18/34 inside its location), resources
    /// (19/35: minerals, gas or both by `score_kind`), kills of its unit
    /// type (20/36) or a score (21/37).
    pub fn leaderboard_value(&self, world: &World, player: usize) -> i64 {
        let Some(lb) = &self.leaderboard else {
            return 0;
        };
        let Some(p) = world.players.get(player) else {
            return 0;
        };
        match lb.kind {
            17 | 18 | 33 | 34 => {
                let c = TriggerCondition {
                    location: lb.location.unwrap_or(0),
                    group: player as u32,
                    unit_id: lb.unit_id,
                    ..TriggerCondition::default()
                };
                if lb.location.is_some() {
                    self.bring_count(world, player, &c) as i64
                } else {
                    self.command_count(world, player, &c) as i64
                }
            }
            19 | 35 => match lb.score_kind {
                0 => p.minerals as i64,
                1 => p.gas as i64,
                _ => (p.minerals + p.gas) as i64,
            },
            20 | 36 => p.kill_counts.get(lb.unit_id as usize).copied().unwrap_or(0) as i64,
            21 | 37 => self.score(world, player, lb.score_kind) as i64,
            _ => 0,
        }
    }

    /// Players of force `force` (0..=3) that are in the game.
    fn force_members(&self, force: usize) -> Vec<usize> {
        (0..8)
            .filter(|&p| self.present[p] && self.forces[p] as usize == force)
            .collect()
    }

    fn select_owners(&self, group: u32, current: usize) -> Vec<usize> {
        match group {
            0..=11 => vec![group as usize],
            12 => (0..12).collect(),
            13 => vec![current],
            14..=17 => (0..12).collect(),
            18..=21 => self.force_members(group as usize - 18),
            _ => vec![current],
        }
    }

    fn loc_center_px(&self, location: u32) -> Option<(i32, i32)> {
        if location == 0 || location == 64 {
            return None;
        }
        self.locations
            .get(location as usize - 1)
            .map(|l| ((l.0 + l.2) / 2, (l.1 + l.3) / 2))
    }

    /// Centre (world px) of location `location` (1-based, as actions name
    /// it); `None` for "Anywhere" (64), 0 or an unknown index.
    pub fn location_center(&self, location: u32) -> Option<(i32, i32)> {
        if location == 0 || location == 64 {
            return None;
        }
        let l = self.locations.get(location as usize - 1)?;
        Some(((l.0 + l.2) / 2, (l.1 + l.3) / 2))
    }

    /// The location of `player`'s last Transmission.
    pub fn last_transmission(&self, player: usize) -> Option<(i32, i32)> {
        self.transmission_locations
            .iter()
            .rev()
            .filter(|(p, _)| *p == player)
            .find_map(|(_, l)| self.location_center(*l))
    }

    fn unit_in_location(&self, x_px: i32, y_px: i32, location: u32) -> bool {
        if location == 0 || location == 64 {
            return true;
        }
        let Some(l) = self.locations.get(location as usize - 1) else {
            return false;
        };
        let x = x_px / 256;
        let y = y_px / 256;
        x >= l.0 && x < l.2 && y >= l.1 && y < l.3
    }

    /// Apply an enable/disable/toggle state byte (4/5/6).
    fn next_state(current: bool, state: u8) -> bool {
        match state {
            4 => true,
            5 => false,
            6 => !current,
            _ => current,
        }
    }

    /// Pick up to `count` matching units (0 means all) in `location`.
    fn collect_targets(
        world: &World,
        owners: &[usize],
        unit: u16,
        count: u32,
        location: u32,
        locations: &[(i32, i32, i32, i32)],
    ) -> Vec<usize> {
        let limit = if count == 0 {
            usize::MAX
        } else {
            count as usize
        };
        let mut out = Vec::new();
        for (i, u) in world.units.iter().enumerate() {
            if !u.alive || !owners.contains(&(u.owner as usize)) {
                continue;
            }
            if !Self::unit_matches(world, u.type_id, unit) {
                continue;
            }
            if location != 0 && location != 64 {
                let Some(l) = locations.get(location as usize - 1) else {
                    continue;
                };
                let x = u.x / 256;
                let y = u.y / 256;
                if !(x >= l.0 && x < l.2 && y >= l.1 && y < l.3) {
                    continue;
                }
            }
            out.push(i);
            if out.len() >= limit {
                break;
            }
        }
        out
    }

    fn next_doodad_state(current: bool, state: u8) -> bool {
        match state {
            0 | 5 => false,
            4 => true,
            6 => !current,
            _ => current,
        }
    }

    fn collect_doodads(
        world: &World,
        unit: u16,
        location: u32,
        locations: &[(i32, i32, i32, i32)],
    ) -> Vec<usize> {
        let mut out = Vec::new();
        for (i, u) in world.units.iter().enumerate() {
            if !u.alive || u.doodad_enabled.is_none() {
                continue;
            }
            if !Self::unit_matches(world, u.type_id, unit) {
                continue;
            }
            if location != 0 && location != 64 {
                let Some(l) = locations.get(location as usize - 1) else {
                    continue;
                };
                let x = u.x / 256;
                let y = u.y / 256;
                if !(x >= l.0 && x < l.2 && y >= l.1 && y < l.3) {
                    continue;
                }
            }
            out.push(i);
            break;
        }
        out
    }

    fn execute(
        &mut self,
        world: &mut World,
        player: usize,
        trigger_index: usize,
        action: &TriggerAction,
    ) {
        match action.kind {
            // Victory and Defeat take no player argument: they end the game
            // for the player running the trigger (CHK `TRIG` spec).  Reading
            // `action.group` (0 in melee maps) defeated player 0 whenever any
            // other player's Defeat trigger fired.
            1 => {
                world.players[player].defeated = false;
                world.winner = Some(player as u8);
            }
            2 => {
                world.players[player].defeated = true;
                for u in &mut world.units {
                    if u.owner as usize == player && !crate::sim::is_resource_type(u.type_id) {
                        u.alive = false;
                        u.order = Order::Die;
                    }
                }
            }
            22 | 24 => {
                let owners = self.select_owners(action.group, player);
                for i in 0..world.units.len() {
                    let type_id = world.units[i].type_id;
                    if world.units[i].alive
                        && owners.contains(&(world.units[i].owner as usize))
                        && Self::unit_matches(world, type_id, action.extra)
                    {
                        world.units[i].alive = false;
                        world.units[i].order = Order::Die;
                    }
                }
            }
            23 | 25 => {
                let owners = self.select_owners(action.group, player);
                let targets = Self::collect_targets(
                    world,
                    &owners,
                    action.extra,
                    action.num as u32,
                    action.location,
                    &self.locations,
                );
                for i in targets {
                    world.units[i].alive = false;
                    world.units[i].order = Order::Die;
                }
            }
            13 => {
                // Set Switch: `group2` is the switch number, `num` the state
                // (4 set, 5 clear, 6 toggle, 11 randomize).
                let sw = action.group2 as usize;
                if sw < self.switches.len() {
                    self.switches[sw] = match action.num {
                        4 => true,
                        5 => false,
                        6 => !self.switches[sw],
                        11 => {
                            let mut x = self.switches[sw] as u64;
                            x = x.wrapping_mul(6364136223846793005).wrapping_add(1);
                            (x >> 33) & 1 != 0
                        }
                        _ => self.switches[sw],
                    };
                }
            }
            14 => {
                // Set Countdown Timer: time, modifier (7 set, 8 add, 9 sub).
                self.countdown_shown = true;
                let t = action.time as i64;
                self.countdown_timer = match action.num {
                    7 => t.max(0) as u32,
                    8 => (self.countdown_timer as i64 + t).max(0) as u32,
                    9 => (self.countdown_timer as i64 - t).max(0) as u32,
                    _ => self.countdown_timer,
                };
            }
            26 => {
                // `num` selects set (7), add (8) or subtract (9); `extra`
                // selects minerals (0), gas (1) or both (2).
                let amount = action.group2 as i32;
                let owners = self.select_owners(action.group, player);
                for o in owners {
                    let Some(p) = world.players.get_mut(o) else {
                        continue;
                    };
                    let apply = |value: &mut i32| match action.num {
                        7 => *value = amount,
                        8 => *value = value.saturating_add(amount),
                        9 => *value = value.saturating_sub(amount),
                        _ => {}
                    };
                    if action.extra == 0 || action.extra == 2 {
                        apply(&mut p.minerals);
                    }
                    if action.extra == 1 || action.extra == 2 {
                        apply(&mut p.gas);
                    }
                }
            }
            38 => {
                // Move Location: shift a named location to a unit/tile.
                let src = action.location.saturating_sub(1) as usize;
                let dst = action.group2.saturating_sub(1) as usize;
                if src < self.locations.len() && dst < self.locations.len() {
                    let old = self.locations[src];
                    let (cx, cy) = ((old.0 + old.2) / 2, (old.1 + old.3) / 2);
                    let (nx, ny) = if action.extra != 0 {
                        world
                            .units
                            .iter()
                            .find(|u| u.alive && u.type_id == action.extra)
                            .map(|u| (u.x / 256, u.y / 256))
                            .unwrap_or((cx, cy))
                    } else {
                        (cx, cy)
                    };
                    let dx = nx - cx;
                    let dy = ny - cy;
                    self.locations[dst].0 += dx;
                    self.locations[dst].2 += dx;
                    self.locations[dst].1 += dy;
                    self.locations[dst].3 += dy;
                }
            }
            11 | 44 => {
                // Unit classes and out-of-range ids create nothing.
                if action.extra > 227 {
                    return;
                }
                let slot = action.group2 as usize;
                let props = if (1..=64).contains(&slot) {
                    self.unit_properties.get(slot - 1).cloned()
                } else {
                    None
                };
                let (x, y) = self.loc_center_px(action.location).unwrap_or((
                    world.terrain.width as i32 * 16,
                    world.terrain.height as i32 * 16,
                ));
                let count = if action.num == 0 {
                    1
                } else {
                    action.num as usize
                };
                for o in self.select_owners(action.group, player) {
                    if o >= world.players.len() {
                        continue;
                    }
                    for _ in 0..count {
                        let spawned = if let Some(props) = props.as_ref() {
                            world.spawn_with_properties(o, action.extra, x, y, props)
                        } else {
                            world.spawn_px(o, action.extra, x, y)
                        };
                        // A single failed placement does not stop the count;
                        // retail tries the nearest free spot for each unit.
                        let _ = spawned;
                    }
                }
            }
            46 => {
                // Order: group=player, extra=unit, from=location,
                // group2=destination, num=0 move / 1 patrol / 2 attack.
                let owners = self.select_owners(action.group, player);
                let targets = Self::collect_targets(
                    world,
                    &owners,
                    action.extra,
                    0,
                    action.location,
                    &self.locations,
                );
                let Some(dst) = self.loc_center_px(action.group2) else {
                    return;
                };
                let ids: Vec<u32> = targets.iter().map(|&i| world.units[i].id).collect();
                match action.num {
                    1 => {
                        for &i in &targets {
                            let from = (world.units[i].x / 256, world.units[i].y / 256);
                            world.units[i].order = Order::Patrol {
                                a: from,
                                b: dst,
                                forward: true,
                            };
                        }
                    }
                    2 => world.command_attack_move(&ids, dst),
                    _ => world.command_move(&ids, dst),
                }
            }
            5 | 6 | 54 | 55 => match action.kind {
                5 => {
                    // Pause Game has no effect for computer players.
                    if world.players.get(player).is_some_and(|p| !p.is_computer) {
                        self.paused = true;
                        self.state.pause_source = Some((player, trigger_index));
                    }
                }
                6 => {
                    self.paused = false;
                    self.state.pause_source = None;
                }
                54 => self.timer_paused = true,
                55 => self.timer_paused = false,
                _ => {}
            },
            15 | 16 => {
                // Run AI Script / Run AI Script At Location.  The four-byte
                // script id is stored in `group2` by StarEdit.
                let mut id = [0u8; 4];
                id.copy_from_slice(&action.group2.to_le_bytes());
                let at = if action.kind == 16 {
                    self.locations
                        .get(action.location.saturating_sub(1) as usize)
                        .map(|l| ((l.0 + l.2) / 2, (l.1 + l.3) / 2))
                } else {
                    None
                };
                self.pending_ai_scripts.push((player, id, at));
            }
            39 => {
                // Move Unit: group=player, extra=unit, num=count,
                // location=from, group2=to.
                let owners = self.select_owners(action.group, player);
                let targets = Self::collect_targets(
                    world,
                    &owners,
                    action.extra,
                    action.num as u32,
                    action.location,
                    &self.locations,
                );
                if let Some((x, y)) = self.loc_center_px(action.group2) {
                    for i in targets {
                        world.units[i].x = x.saturating_mul(256);
                        world.units[i].y = y.saturating_mul(256);
                        world.units[i].path.clear();
                    }
                }
            }
            45 => {
                // Set Deaths: group=player, extra=unit, group2=amount,
                // num=modifier (7 set, 8 add, 9 subtract).  Unit classes
                // 230..=232 write the real loss counters; Any unit writes
                // nothing (retail behaviour).
                let owners = self.select_owners(action.group, player);
                let n = action.group2;
                for o in owners {
                    let Some(p) = world.players.get_mut(o) else {
                        continue;
                    };
                    if action.extra <= 227 {
                        let idx = action.extra as usize;
                        if idx < p.death_counts.len() {
                            let v = &mut p.death_counts[idx];
                            match action.num {
                                7 => *v = n,
                                8 => *v = v.saturating_add(n),
                                9 => *v = v.saturating_sub(n),
                                _ => {}
                            }
                        }
                    } else if action.extra == 230 || action.extra == 231 || action.extra == 232 {
                        let v = &mut p.deaths;
                        match action.num {
                            7 => *v = n,
                            8 => *v = v.saturating_add(n),
                            9 => *v = v.saturating_sub(n),
                            _ => {}
                        }
                    }
                }
            }
            48 => {
                // Give Units to Player: group=from, group2=to, extra=unit,
                // num=count, location.  A group "from" applies per player.
                let sources = self.select_owners(action.group, player);
                let Some(dst) = self.select_owners(action.group2, player).first().copied() else {
                    return;
                };
                let targets = Self::collect_targets(
                    world,
                    &sources,
                    action.extra,
                    action.num as u32,
                    action.location,
                    &self.locations,
                );
                for i in targets {
                    world.units[i].owner = dst as u8;
                }
            }
            49 | 50 | 51 | 53 => {
                let owners = self.select_owners(action.group, player);
                let targets = Self::collect_targets(
                    world,
                    &owners,
                    action.extra,
                    action.num as u32,
                    action.location,
                    &self.locations,
                );
                let percent = action.group2.min(100) as i32;
                for i in targets {
                    match action.kind {
                        49 => {
                            let max = world.data.unit(world.units[i].type_id).hit_points_raw as i32;
                            world.units[i].hp = (max * percent / 100).max(1);
                        }
                        50 => {
                            let cap = world.units[i].energy_cap;
                            world.units[i].set_energy_fixed(cap * percent / 100);
                        }
                        51 => {
                            let ty = world.data.unit(world.units[i].type_id);
                            if ty.has_shields {
                                world.units[i].shields = (ty.shield_points as i32 * percent).max(0);
                            }
                        }
                        53 => {
                            let amount = action.group2.min(255) as u8;
                            let cap = world.units[i].hangar_capacity;
                            world.units[i].hangar =
                                world.units[i].hangar.saturating_add(amount).min(cap);
                        }
                        _ => {}
                    }
                }
            }
            52 => {
                let owners = self.select_owners(action.group, player);
                let amount = (action.group2 as u16) as i32;
                for u in &mut world.units {
                    if u.alive
                        && owners.contains(&(u.owner as usize))
                        && u.is_resource()
                        && self.unit_in_location(u.x, u.y, action.location)
                    {
                        u.resource_amount = amount;
                    }
                }
            }
            7 => {
                let ms = transmission_ms(action);
                self.transmissions
                    .push((player, action.extra, action.text_id, ms));
                self.transmission_locations.push((player, action.location));
                self.cues.push(TriggerCue::Wav {
                    player,
                    string: action.sound_id,
                });
                self.cues.push(TriggerCue::Text {
                    player,
                    string: action.text_id,
                    always: action.flags & ALWAYS_DISPLAY != 0,
                    ms,
                });
            }
            8 => {
                self.cues.push(TriggerCue::Wav {
                    player,
                    string: action.sound_id,
                });
            }
            9 => {
                self.cues.push(TriggerCue::Text {
                    player,
                    string: action.text_id,
                    always: action.flags & ALWAYS_DISPLAY != 0,
                    ms: 0,
                });
            }
            10 => {
                self.center_views.push((player, action.location));
            }
            12 => {
                if self.mission_objectives.len() <= player {
                    self.mission_objectives.resize(player + 1, None);
                }
                self.mission_objectives[player] = Some(action.text_id);
            }
            17..=21 | 33..=37 => {
                self.leaderboard = Some(Leaderboard {
                    player,
                    text_id: action.text_id,
                    kind: action.kind,
                    unit_id: action.extra,
                    score_kind: action.extra,
                    goal: (33..=37).contains(&action.kind).then_some(action.group2),
                    location: matches!(action.kind, 18 | 34).then_some(action.location),
                });
            }
            27 => {
                // Set Score: group=player, group2=amount, extra=score kind,
                // num=modifier.
                let targets = self.select_owners(action.group, player);
                let amount = action.group2 as i32;
                let kind = action.extra as usize;
                let apply = |slot: &mut i32, share: i32| match action.num {
                    7 => *slot = share,
                    8 => *slot = slot.saturating_add(share),
                    9 => *slot = slot.saturating_sub(share),
                    _ => {}
                };
                for o in targets {
                    let Some(p) = world.players.get_mut(o) else {
                        continue;
                    };
                    match kind {
                        0 => {
                            let share = amount / 4;
                            apply(&mut p.scores[1], share);
                            apply(&mut p.scores[2], share);
                            apply(&mut p.scores[4], share);
                            apply(&mut p.scores[5], share);
                        }
                        3 => {
                            let share = amount / 2;
                            apply(&mut p.scores[1], share);
                            apply(&mut p.scores[2], share);
                        }
                        6 => {
                            let share = amount / 2;
                            apply(&mut p.scores[4], share);
                            apply(&mut p.scores[5], share);
                        }
                        1..=5 | 7 => apply(&mut p.scores[kind], amount),
                        _ => {}
                    }
                }
            }
            28 => {
                self.minimap_pings.push((player, action.location));
            }
            29 => {
                self.talking_portraits
                    .push((player, action.extra, action.time));
            }
            30 => self.muted_speech = true,
            31 => self.muted_speech = false,
            32 => {
                self.leaderboard_computer_players =
                    Self::next_state(self.leaderboard_computer_players, action.num);
            }
            40 => {
                self.leaderboard = Some(Leaderboard {
                    player,
                    text_id: action.text_id,
                    kind: 40,
                    unit_id: 0,
                    score_kind: 0,
                    goal: Some(action.group2),
                    location: None,
                });
            }
            41 => self.next_scenario = Some(action.text_id.saturating_sub(1)),
            42 => {
                let targets =
                    Self::collect_doodads(world, action.extra, action.location, &self.locations);
                if let Some(&i) = targets.first() {
                    let current = world.units[i].doodad_enabled.unwrap_or(false);
                    world.units[i].doodad_enabled =
                        Some(Self::next_doodad_state(current, action.num));
                }
            }
            43 => {
                let owners = self.select_owners(action.group, player);
                let targets = Self::collect_targets(
                    world,
                    &owners,
                    action.extra,
                    0,
                    action.location,
                    &self.locations,
                );
                for i in targets {
                    world.units[i].invincible =
                        Self::next_state(world.units[i].invincible, action.num);
                }
            }
            58 | 59 => {
                // Enable/Disable Debug Mode (Remastered): controls whether Esc
                // can skip a trigger.  There is no renderer or Esc key here.
            }
            56 => {
                // Draw: no winner.
                world.winner = None;
            }
            57 => {
                // Set Alliance Status: `group` is the Player argument and
                // `group2` is unused (zero in every shipped use).  `extra`
                // is 0 enemy / 1 ally / 2 allied victory; any other value
                // is an enemy.
                let old_team = world.players[player].team;
                let ally = action.extra == 1 || action.extra == 2;
                let mut matched = false;
                for target in self.select_owners(action.group, player) {
                    if target >= world.players.len() || target == player {
                        continue;
                    }
                    if ally {
                        world.players[target].team = old_team;
                    } else if world.players[target].team == old_team {
                        world.players[target].team = old_team.wrapping_add(1);
                    }
                    matched = true;
                }
                if matched {
                    world.players[player].allied_victory = action.extra == 2;
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn transmission_duration_applies_the_modifier() {
        let a = |num, time, group2| TriggerAction {
            num,
            time,
            group2,
            ..Default::default()
        };
        assert_eq!(super::transmission_ms(&a(7, 2000, 500)), 500);
        assert_eq!(super::transmission_ms(&a(8, 2000, 500)), 2500);
        assert_eq!(super::transmission_ms(&a(9, 2000, 500)), 1500);
        assert_eq!(super::transmission_ms(&a(9, 200, 500)), 0);
        assert_eq!(super::transmission_ms(&a(0, 2000, 500)), 2000);
    }

    use super::*;

    fn record(kind: u8, action: u8, enabled: usize) -> Vec<u8> {
        let mut r = vec![0u8; RECORD_SIZE];
        r[15] = kind;
        // The zero quantity is the neutral default for always/countdown tests.
        r[8..12].copy_from_slice(&0u32.to_le_bytes());
        // action 0
        r[320 + 26] = action;
        r[320 + 27] = 0;
        r[2372 + enabled] = 1;
        let _ = kind;
        r
    }

    #[test]
    fn space_finds_the_last_transmission_location() {
        // "Space Bar - Last Transmission" (help_txt): the Transmission's
        // location is kept for the client.
        let mut r = record(22, 7, 0);
        let o = 320;
        r[o..o + 4].copy_from_slice(&1u32.to_le_bytes()); // location 1
        r[o + 4..o + 8].copy_from_slice(&5u32.to_le_bytes());
        r[o + 12..o + 16].copy_from_slice(&2000u32.to_le_bytes());
        r[o + 24..o + 26].copy_from_slice(&1u16.to_le_bytes());
        r[o + 26] = 7;
        let mut t = TriggerSystem::parse(&r, &[(100, 200, 300, 400)]);
        let mut w = world();
        assert_eq!(t.last_transmission(0), None);
        t.process(&mut w);
        assert_eq!(t.last_transmission(0), Some((200, 300)));
        assert_eq!(t.last_transmission(1), None, "only the receiving player");
        assert_eq!(t.location_center(64), None, "Anywhere has no centre");
    }

    #[test]
    fn transmission_play_wav_and_display_text_log_ordered_cues() {
        // One Always trigger: Transmission (text 5, wav 6, 2 s, flag
        // "always display"), Play WAV (wav 7), Display Text (text 8).
        let mut r = record(22, 7, 0);
        let act = |r: &mut Vec<u8>, i: usize, kind: u8, text: u32, wav: u32, flags: u8| {
            let o = 320 + 32 * i;
            r[o + 4..o + 8].copy_from_slice(&text.to_le_bytes());
            r[o + 8..o + 12].copy_from_slice(&wav.to_le_bytes());
            r[o + 12..o + 16].copy_from_slice(&2000u32.to_le_bytes());
            r[o + 24..o + 26].copy_from_slice(&1u16.to_le_bytes()); // Marine
            r[o + 26] = kind;
            r[o + 28] = flags;
        };
        act(&mut r, 0, 7, 5, 6, ALWAYS_DISPLAY);
        act(&mut r, 1, 8, 0, 7, 0);
        act(&mut r, 2, 9, 8, 0, 0);
        let mut t = TriggerSystem::parse(&r, &[]);
        let mut w = world();
        t.process(&mut w);
        // The transmission holds the trigger for its 2 s line.
        assert_eq!(t.cues.len(), 2, "only the transmission so far");
        w.tick += 60;
        t.process(&mut w);
        assert_eq!(
            t.cues,
            vec![
                TriggerCue::Wav {
                    player: 0,
                    string: 6
                },
                TriggerCue::Text {
                    player: 0,
                    string: 5,
                    always: true,
                    ms: 2000,
                },
                TriggerCue::Wav {
                    player: 0,
                    string: 7
                },
                TriggerCue::Text {
                    player: 0,
                    string: 8,
                    always: false,
                    ms: 0,
                },
            ]
        );
        assert_eq!(
            t.transmissions,
            vec![(0, 1, 5, 2000)],
            "portrait record kept"
        );
        assert!(t.cues.iter().all(|c| c.player() == 0));
    }

    #[test]
    fn every_classic_action_id_is_accepted() {
        // The CHK action list runs 0..=59.  Feed each id a fully populated
        // record so missing operands cannot mask a panic.
        for action in 1..=59u8 {
            let mut r = record(22, action, 0);
            r[320..320 + 4].copy_from_slice(&1u32.to_le_bytes()); // location
            r[320 + 4..320 + 8].copy_from_slice(&1u32.to_le_bytes()); // text
            r[320 + 8..320 + 12].copy_from_slice(&1u32.to_le_bytes()); // wav
            r[320 + 12..320 + 16].copy_from_slice(&500u32.to_le_bytes()); // time
            r[320 + 16..320 + 20].copy_from_slice(&0u32.to_le_bytes()); // player
            r[320 + 20..320 + 24].copy_from_slice(&1u32.to_le_bytes()); // group2
            r[320 + 24..320 + 26].copy_from_slice(&1u16.to_le_bytes()); // extra
            r[320 + 27] = 8;
            let mut t = TriggerSystem::parse(&r, &[]);
            let mut w = world();
            t.process(&mut w);
        }
    }

    #[test]
    fn every_classic_condition_id_is_accepted() {
        for kind in 0..=23u8 {
            let mut r = record(kind, 26, 0);
            r[8..12].copy_from_slice(&0u32.to_le_bytes());
            r[14] = 0;
            r[16] = 0;
            r[320 + 20..320 + 24].copy_from_slice(&1u32.to_le_bytes());
            r[320 + 27] = 8;
            let mut t = TriggerSystem::parse(&r, &[]);
            let mut w = world();
            t.process(&mut w);
        }
    }

    #[test]
    fn shipped_melee_trigger_uses_command_and_opponents() {
        let t = TriggerSystem::parse(embedded!("fixtures/melee.trg"), &[]);
        assert_eq!(t.triggers.len(), 2);
        assert_eq!(t.triggers[0].conditions[0].kind, 2); // command
        assert_eq!(t.triggers[0].actions[0].kind, 2); // defeat
        assert_eq!(t.triggers[1].conditions[0].kind, 14); // opponents
        assert_eq!(t.triggers[1].actions[0].kind, 1); // victory
        assert!(t.triggers[0].enabled[17], "melee enables all players");
    }

    #[test]
    fn parses_a_single_trigger_record() {
        let t = TriggerSystem::parse(&record(15, 2, 0), &[]);
        assert_eq!(t.triggers.len(), 1);
        assert_eq!(t.triggers[0].conditions.len(), 1);
        assert_eq!(t.triggers[0].actions.len(), 1);
        assert!(t.triggers[0].enabled[0]);
    }

    fn world() -> World {
        use crate::data::GameData;
        use crate::map::Terrain;
        use std::sync::Arc;
        let mut w = World::new(
            Arc::new(GameData::bundled().unwrap()),
            Terrain::open(16, 16),
            2,
        );
        w.tick = 30;
        w
    }

    #[test]
    fn set_resources_action_honours_the_operation_and_resource_mask() {
        let mut r = record(22, 26, 0);
        // group2 = 500, extra = 2 (both), num = 7 (set)
        r[320 + 20..320 + 24].copy_from_slice(&500u32.to_le_bytes());
        r[320 + 24] = 2;
        r[320 + 27] = 7;
        let mut t = TriggerSystem::parse(&r, &[]);
        let mut w = world();
        t.process(&mut w);
        assert_eq!(w.players[0].minerals, 500);
        assert_eq!(w.players[0].gas, 500);

        // Add 25 gas, leave minerals untouched.
        let mut r = record(22, 26, 0);
        r[320 + 20..320 + 24].copy_from_slice(&25u32.to_le_bytes());
        r[320 + 24] = 1;
        r[320 + 27] = 8;
        let mut t = TriggerSystem::parse(&r, &[]);
        let mut w = world();
        w.players[0].gas = 10;
        t.process(&mut w);
        assert_eq!(w.players[0].minerals, 50);
        assert_eq!(w.players[0].gas, 35);
    }

    #[test]
    fn set_switch_action_feeds_the_switch_condition() {
        // First trigger: always -> set switch #3.
        let mut a = record(22, 13, 0);
        a[320 + 20..320 + 24].copy_from_slice(&3u32.to_le_bytes());
        a[320 + 27] = 4; // set
        // Second trigger: switch #3 is set -> set minerals to 123.
        let mut b = record(22, 26, 0);
        b[15] = 11;
        b[14] = 2; // switch is set
        b[16] = 3; // switch number
        b[320 + 20..320 + 24].copy_from_slice(&123u32.to_le_bytes());
        b[320 + 24] = 0;
        b[320 + 27] = 7;
        let mut bytes = a;
        bytes.extend_from_slice(&b);
        let mut t = TriggerSystem::parse(&bytes, &[]);
        let mut w = world();
        t.process(&mut w);
        assert!(t.switches[3]);
        assert_eq!(w.players[0].minerals, 123);
    }

    #[test]
    fn next_scenario_text_id_is_stored_one_greater_than_the_str_index() {
        // Every shipped action-41 use points one past the `.scm` string:
        // Enslavers 01 stores 21 for string 20, 02a stores 16 for string 15.
        let mut r = record(22, 41, 0);
        r[320 + 4..320 + 8].copy_from_slice(&17u32.to_le_bytes());
        let mut t = TriggerSystem::parse(&r, &[]);
        let mut w = world();
        t.process(&mut w);
        assert_eq!(t.next_scenario, Some(16));
    }

    #[test]
    fn elapsed_time_condition_is_in_seconds() {
        let mut r = record(12, 26, 0);
        r[15] = 12;
        r[8..12].copy_from_slice(&30u32.to_le_bytes());
        r[320 + 20..320 + 24].copy_from_slice(&5u32.to_le_bytes());
        r[320 + 24] = 0;
        r[320 + 27] = 7;
        let mut t = TriggerSystem::parse(&r, &[]);
        let mut w = world(); // tick 30 = 1 second
        t.process(&mut w);
        assert_eq!(w.players[0].minerals, 50, "1 second is not 30 seconds");
        w.tick = 30 * 30;
        t.process(&mut w);
        assert_eq!(w.players[0].minerals, 5);
    }

    fn add_action(rec: &mut [u8], slot: usize, kind: u8) -> usize {
        let o = 320 + slot * 32;
        rec[o + 26] = kind;
        o
    }

    #[test]
    fn exactly_comparison_is_the_chk_value_ten() {
        // First trigger: always -> Set Score Custom 5.
        let mut a = record(22, 27, 0);
        a[320 + 20..320 + 24].copy_from_slice(&5u32.to_le_bytes());
        a[320 + 24..320 + 26].copy_from_slice(&7u16.to_le_bytes()); // custom
        a[320 + 27] = 7; // set
        // Second trigger: Score(Custom) Exactly 5 -> minerals 77.
        let mut b = record(21, 26, 0);
        b[15] = 21;
        b[8..12].copy_from_slice(&5u32.to_le_bytes());
        b[14] = 10; // Exactly
        b[16] = 7; // Custom
        b[320 + 20..320 + 24].copy_from_slice(&77u32.to_le_bytes());
        b[320 + 24] = 0;
        b[320 + 27] = 7;
        let mut bytes = a;
        bytes.extend_from_slice(&b);
        let mut t = TriggerSystem::parse(&bytes, &[]);
        let mut w = world();
        t.process(&mut w);
        assert_eq!(w.players[0].scores[7], 5);
        assert_eq!(w.players[0].minerals, 77);
    }

    #[test]
    fn create_unit_with_properties_applies_uprp_slot_to_every_unit() {
        let mut props = vec![0u8; 64 * 20];
        props[2..4].copy_from_slice(&0x12u16.to_le_bytes()); // HP + resources
        props[5] = 50;
        props[8..12].copy_from_slice(&123u32.to_le_bytes());
        props[0..2].copy_from_slice(&0x10u16.to_le_bytes()); // invincible state mask
        props[14..16].copy_from_slice(&0x10u16.to_le_bytes()); // invincible
        let mut chk = Vec::new();
        let mut push = |name: &[u8; 4], bytes: &[u8]| {
            chk.extend_from_slice(name);
            chk.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            chk.extend_from_slice(bytes);
        };
        let mut dim = Vec::new();
        dim.extend_from_slice(&4u16.to_le_bytes());
        dim.extend_from_slice(&4u16.to_le_bytes());
        push(b"DIM ", &dim);
        push(b"ERA ", &0u16.to_le_bytes());
        let mut mtxm = vec![0u8; 4 * 4 * 2];
        for i in 0..16 {
            mtxm[i * 2..i * 2 + 2].copy_from_slice(&32u16.to_le_bytes());
        }
        push(b"MTXM", &mtxm);
        push(b"MASK", &[1u8; 16]);
        push(b"UPRP", &props);
        let mut uprp = crate::map::Scenario::parse(&chk).unwrap().unit_properties;

        let mut r = record(22, 11, 0);
        r[320 + 16..320 + 20].copy_from_slice(&0u32.to_le_bytes()); // player
        r[320 + 20..320 + 24].copy_from_slice(&1u32.to_le_bytes()); // slot 1
        r[320 + 24..320 + 26].copy_from_slice(&crate::ids::unit::MARINE.to_le_bytes());
        r[320 + 27] = 2; // two units
        let mut t = TriggerSystem::parse(&r, &[]);
        t.unit_properties = std::mem::take(&mut uprp);
        let mut w = world();
        t.process(&mut w);
        let marines: Vec<_> = w
            .units
            .iter()
            .filter(|u| u.alive && u.type_id == crate::ids::unit::MARINE)
            .collect();
        assert_eq!(marines.len(), 2);
        let max = w.data.unit(crate::ids::unit::MARINE).hit_points_raw as i32;
        for u in marines {
            assert_eq!(u.hp, (max * 50 / 100).max(1));
            assert_eq!(
                u.resource_amount, 0,
                "only units.dat flag 0x2000 takes resources"
            );
            assert!(u.invincible);
        }
    }

    #[test]
    fn create_unit_count_and_kill_unit_at_location_count_are_honoured() {
        let locations = vec![(0, 0, 64, 64)];
        let mut r = record(22, 44, 0);
        r[320..320 + 4].copy_from_slice(&1u32.to_le_bytes());
        r[320 + 16..320 + 20].copy_from_slice(&0u32.to_le_bytes());
        r[320 + 20..320 + 24].copy_from_slice(&0u32.to_le_bytes());
        r[320 + 24..320 + 26].copy_from_slice(&crate::ids::unit::MARINE.to_le_bytes());
        r[320 + 27] = 3;
        let mut t = TriggerSystem::parse(&r, &locations);
        let mut w = world();
        t.process(&mut w);
        assert_eq!(
            w.units
                .iter()
                .filter(|u| u.alive && u.type_id == crate::ids::unit::MARINE)
                .count(),
            3
        );

        let mut k = record(22, 23, 0);
        k[320..320 + 4].copy_from_slice(&1u32.to_le_bytes());
        k[320 + 16..320 + 20].copy_from_slice(&0u32.to_le_bytes());
        k[320 + 24..320 + 26].copy_from_slice(&crate::ids::unit::MARINE.to_le_bytes());
        k[320 + 27] = 2;
        let mut t2 = TriggerSystem::parse(&k, &locations);
        t2.process(&mut w);
        assert_eq!(
            w.units
                .iter()
                .filter(|u| u.alive && u.type_id == crate::ids::unit::MARINE)
                .count(),
            1
        );
    }

    #[test]
    fn modify_hit_points_uses_a_percentage_not_a_delta() {
        let mut r = record(22, 49, 0);
        r[320 + 16..320 + 20].copy_from_slice(&0u32.to_le_bytes());
        r[320 + 20..320 + 24].copy_from_slice(&50u32.to_le_bytes()); // 50%
        r[320 + 24..320 + 26].copy_from_slice(&crate::ids::unit::MARINE.to_le_bytes());
        let mut t = TriggerSystem::parse(&r, &[]);
        let mut w = world();
        let id = w.spawn_px(0, crate::ids::unit::MARINE, 64, 64).unwrap();
        let max = w.data.unit(crate::ids::unit::MARINE).hit_points_raw as i32;
        w.units[id].hp = max;
        t.process(&mut w);
        assert_eq!(w.units[id].hp, max * 50 / 100);
    }

    #[test]
    fn set_invincibility_blocks_damage_until_disabled() {
        let mut r = record(22, 43, 0);
        r[320 + 16..320 + 20].copy_from_slice(&0u32.to_le_bytes());
        r[320 + 24..320 + 26].copy_from_slice(&crate::ids::unit::MARINE.to_le_bytes());
        r[320 + 27] = 4; // enable
        let mut t = TriggerSystem::parse(&r, &[]);
        let mut w = world();
        let id = w.spawn_px(0, crate::ids::unit::MARINE, 64, 64).unwrap();
        let before = w.units[id].hp;
        t.process(&mut w);
        assert!(w.units[id].invincible);
        w.damage_unit(w.units[id].id, 1_000).unwrap();
        assert_eq!(w.units[id].hp, before);

        // Toggle off, then damage lands.
        let mut r2 = record(22, 43, 0);
        r2[320 + 24..320 + 26].copy_from_slice(&crate::ids::unit::MARINE.to_le_bytes());
        r2[320 + 27] = 5; // disable
        let mut t2 = TriggerSystem::parse(&r2, &[]);
        t2.process(&mut w);
        assert!(!w.units[id].invincible);
        w.damage_unit(w.units[id].id, 100).unwrap();
        assert!(w.units[id].hp < before);
    }

    #[test]
    fn force_triggers_run_for_the_force_members_in_the_game() {
        // A trigger enabled only for Force 1 (index 18) that sets minerals.
        let mut r = vec![0u8; RECORD_SIZE];
        r[15] = 22; // Always
        r[320 + 20..320 + 24].copy_from_slice(&777u32.to_le_bytes());
        r[320 + 16..320 + 20].copy_from_slice(&13u32.to_le_bytes()); // current player
        r[320 + 26] = 26; // Set Resources
        r[320 + 27] = 7; // set to
        r[2372 + 18] = 1;
        let mut t = TriggerSystem::parse(&r, &[]);
        // Player 0 in force 1 (index 0), player 1 in force 2.
        t.forces = [0, 1, 0, 0, 0, 0, 0, 0];
        t.present = [true, true, false, false, false, false, false, false];
        let mut w = world();
        w.tick = 1;
        t.process(&mut w);
        assert_eq!(w.players[0].minerals, 777, "force 1 member runs it");
        assert_ne!(w.players[1].minerals, 777, "force 2 does not");
        // Before the fix only per-player (0..8) and All Players (17)
        // triggers ran, so Force 1 campaign triggers never executed.
    }

    #[test]
    fn triggers_first_run_at_elapsed_time_zero() {
        // `terran04`: Elapsed Time at most 0 -> set objectives.
        let mut r = record(12, 26, 0);
        r[13] = 1; // at most
        r[8..12].copy_from_slice(&0u32.to_le_bytes());
        r[320 + 20..320 + 24].copy_from_slice(&5u32.to_le_bytes());
        r[320 + 27] = 7;
        let mut t = TriggerSystem::parse(&r, &[]);
        let mut w = world();
        w.tick = 1;
        t.process(&mut w);
        assert_eq!(w.players[0].minerals, 5);
    }

    #[test]
    fn a_wait_resumes_on_the_tick_it_ends() {
        // Wait 2000 ms (48 ticks) then set minerals: retail resumes at
        // the end of the wait, not at the next 30-tick trigger cycle.
        let mut r = record(22, 4, 0);
        r[320 + 12..320 + 16].copy_from_slice(&2000u32.to_le_bytes());
        let a1 = add_action(&mut r, 1, 26);
        r[a1 + 20..a1 + 24].copy_from_slice(&777u32.to_le_bytes());
        r[a1 + 27] = 7;
        let mut t = TriggerSystem::parse(&r, &[]);
        let mut w = world();
        w.tick = 1;
        t.process(&mut w);
        for tick in 2..=48 {
            w.tick = tick;
            t.process(&mut w);
            assert_ne!(w.players[0].minerals, 777, "still waiting at {tick}");
        }
        w.tick = 49;
        t.process(&mut w);
        assert_eq!(w.players[0].minerals, 777, "resumed at tick 1 + 48");
    }

    #[test]
    fn victory_and_defeat_apply_to_the_player_running_the_trigger() {
        // Always -> Victory, enabled for player 2 (index 1), group 0.
        let mut t = TriggerSystem::parse(&record(22, 1, 1), &[]);
        let mut w = world();
        w.tick = 1;
        t.process(&mut w);
        assert_eq!(
            w.winner,
            Some(1),
            "the trigger's player wins, not player 1 (group 0)"
        );
        let mut t = TriggerSystem::parse(&record(22, 2, 1), &[]);
        let mut w = world();
        w.tick = 1;
        t.process(&mut w);
        assert!(w.players[1].defeated && !w.players[0].defeated);
    }

    #[test]
    fn a_control_leaderboard_counts_the_players_units() {
        // Leader Board (Control) of Marines (unit 0), as Wasteland's.
        let mut r = record(22, 17, 0);
        r[320 + 24..320 + 26].copy_from_slice(&0u16.to_le_bytes());
        let mut t = TriggerSystem::parse(&r, &[]);
        let mut w = world();
        for x in [100, 140, 180] {
            w.spawn_px(0, crate::ids::unit::MARINE, x, 100).unwrap();
        }
        w.spawn_px(1, crate::ids::unit::MARINE, 400, 400).unwrap();
        w.tick = 1;
        t.process(&mut w);
        assert_eq!(t.leaderboard_value(&w, 0), 3);
        assert_eq!(t.leaderboard_value(&w, 1), 1);
    }

    #[test]
    fn a_wait_holds_the_players_later_triggers_in_the_same_cycle() {
        // Trigger 0: Always -> Wait 1000. Trigger 1: Always -> 777 minerals.
        let mut r = record(22, 4, 0);
        r[320 + 12..320 + 16].copy_from_slice(&1000u32.to_le_bytes());
        let mut r2 = record(22, 26, 0);
        r2[320 + 20..320 + 24].copy_from_slice(&777u32.to_le_bytes());
        r2[320 + 27] = 7;
        r.extend_from_slice(&r2);
        let mut t = TriggerSystem::parse(&r, &[]);
        let mut w = world();
        w.tick = 1;
        t.process(&mut w);
        assert_ne!(w.players[0].minerals, 777, "held by the wait of trigger 0");
        for tick in 2..=60 {
            w.tick = tick;
            t.process(&mut w);
        }
        assert_eq!(
            w.players[0].minerals, 777,
            "runs on the next cycle after the wait"
        );
    }

    #[test]
    fn the_countdown_and_elapsed_time_run_on_16_tick_game_seconds() {
        // Retail Desperate Alliance: 446-447 countdown seconds per 300 real
        // seconds at 23.81 ticks/s, i.e. one per 16 ticks.
        let mut t = TriggerSystem::parse(&record(22, 0, 0), &[]);
        let mut w = world();
        t.countdown_timer = 1800;
        for tick in 1..=24 * 300 {
            w.tick = tick;
            t.process(&mut w);
        }
        assert_eq!(1800 - t.countdown_timer, 450, "7200 ticks / 16");
        // Elapsed Time at least 10 game seconds: true from tick 160.
        let c = TriggerCondition {
            kind: 12,
            comparison: 0,
            quantity: 10,
            ..TriggerCondition::default()
        };
        w.tick = 159;
        assert!(!t.condition_holds(&w, 0, &c));
        w.tick = 160;
        assert!(t.condition_holds(&w, 0, &c), "160 ticks = 10 game seconds");
    }

    #[test]
    fn set_alliance_status_stops_two_players_fighting() {
        // Shipped layout: Player argument is `group` (player 2); `group2`
        // is unused; extra 1 allies the target with the trigger's player.
        let mut r = record(22, 57, 0);
        r[320 + 16..320 + 20].copy_from_slice(&1u32.to_le_bytes());
        r[320 + 20..320 + 24].copy_from_slice(&0u32.to_le_bytes());
        r[320 + 24..320 + 26].copy_from_slice(&1u16.to_le_bytes());
        let mut t = TriggerSystem::parse(&r, &[]);
        let mut w = world();
        w.players[1].team = 1;
        assert!(w.hostile(0, 1), "enemies before the trigger");
        w.tick = 1;
        t.process(&mut w);
        assert!(!w.hostile(0, 1) && !w.hostile(1, 0), "allied after it");
    }

    #[test]
    fn transmissions_play_one_after_another() {
        // Two 3 s transmissions in one trigger: the second waits for the
        // first line to finish instead of starting in the same tick.
        let mut r = record(22, 7, 0);
        r[320 + 12..320 + 16].copy_from_slice(&3000u32.to_le_bytes());
        let a1 = add_action(&mut r, 1, 7);
        r[a1 + 12..a1 + 16].copy_from_slice(&3000u32.to_le_bytes());
        let mut t = TriggerSystem::parse(&r, &[]);
        let mut w = world();
        t.process(&mut w);
        assert_eq!(t.transmissions.len(), 1);
        w.tick += 30;
        t.process(&mut w);
        assert_eq!(t.transmissions.len(), 1, "still talking at 1.25 s");
        w.tick += 60;
        t.process(&mut w);
        assert_eq!(t.transmissions.len(), 2, "next line after 3 s");
    }

    #[test]
    fn wait_action_resumes_the_remaining_actions() {
        // Actions: set minerals 10, wait 0 ms, set gas 20.
        let mut r = record(22, 26, 0);
        let a0 = 320;
        r[a0 + 20..a0 + 24].copy_from_slice(&10u32.to_le_bytes());
        r[a0 + 24] = 0;
        r[a0 + 27] = 7;
        let a1 = add_action(&mut r, 1, 4);
        r[a1 + 12..a1 + 16].copy_from_slice(&0u32.to_le_bytes());
        let a2 = add_action(&mut r, 2, 26);
        r[a2 + 20..a2 + 24].copy_from_slice(&20u32.to_le_bytes());
        r[a2 + 24] = 1;
        r[a2 + 27] = 7;
        let mut t = TriggerSystem::parse(&r, &[]);
        let mut w = world();
        t.process(&mut w);
        assert_eq!(w.players[0].minerals, 10);
        assert_eq!(w.players[0].gas, 0);
        w.tick += 30;
        t.process(&mut w);
        assert_eq!(w.players[0].gas, 20);
        assert_eq!(w.players[0].minerals, 10, "wait resumed, not restarted");
    }

    #[test]
    fn set_doodad_state_and_order_action_apply_the_chosen_mode() {
        let locations = vec![(0, 0, 64, 64), (64, 64, 128, 128)];
        let mut r = record(22, 42, 0);
        r[320..320 + 4].copy_from_slice(&1u32.to_le_bytes());
        r[320 + 24..320 + 26].copy_from_slice(&0xcdu16.to_le_bytes());
        r[320 + 27] = 5; // disable/open
        let mut t = TriggerSystem::parse(&r, &locations);
        let mut w = world();
        let door = w.spawn_px(0, 0xcd, 32, 32).unwrap();
        w.units[door].doodad_enabled = Some(true);
        t.process(&mut w);
        assert_eq!(w.units[door].doodad_enabled, Some(false));
        let id = w.spawn_px(0, crate::ids::unit::MARINE, 32, 32).unwrap();

        let mut o = record(22, 46, 0);
        o[320..320 + 4].copy_from_slice(&1u32.to_le_bytes());
        o[320 + 16..320 + 20].copy_from_slice(&0u32.to_le_bytes());
        o[320 + 20..320 + 24].copy_from_slice(&2u32.to_le_bytes());
        o[320 + 24..320 + 26].copy_from_slice(&crate::ids::unit::MARINE.to_le_bytes());
        o[320 + 27] = 1; // patrol
        let mut t2 = TriggerSystem::parse(&o, &locations);
        t2.process(&mut w);
        assert!(matches!(w.units[id].order, Order::Patrol { .. }));

        o[320 + 27] = 2; // attack
        let mut t3 = TriggerSystem::parse(&o, &locations);
        t3.process(&mut w);
        assert!(matches!(w.units[id].order, Order::AttackMove { .. }));
    }

    #[test]
    fn every_classic_action_id_dispatches_without_silently_dropping_gameplay_ids() {
        // The semantic ids covered by this pass must reach their handler even
        // when the operands are the neutral defaults.
        for kind in [
            7u8, 8, 9, 10, 11, 12, 17, 18, 19, 20, 21, 23, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36,
            37, 40, 41, 42, 43, 44, 46, 49, 50, 51, 52, 53, 54, 55, 57,
        ] {
            let mut r = record(22, kind, 0);
            r[320 + 16..320 + 20].copy_from_slice(&0u32.to_le_bytes());
            r[320 + 20..320 + 24].copy_from_slice(&1u32.to_le_bytes());
            r[320 + 24..320 + 26].copy_from_slice(&crate::ids::unit::MARINE.to_le_bytes());
            r[320 + 27] = 4;
            let mut t = TriggerSystem::parse(&r, &[]);
            let mut w = world();
            let _ = w.spawn_px(0, crate::ids::unit::MARINE, 64, 64);
            t.process(&mut w);
        }
    }
}

// ===========================================================================
// Merged from src/wave13/agent05.rs
// ===========================================================================
// Wave 13 batch 05 — the trigger *accumulate* condition family.
//
// Window `0x664000`–`0x665fff` (StarCraft 1.23.10).  Every routine in this
// window answers the same question: *how many units of a given kind belong
// to the player set denoted by a selector?*  The answer is either read from
// per-player/per-kind count tables or summed over the player slots that the
// selector denotes.
//
// ## Recovered player selectors (first argument)
//
// | selector | slot set |
// |---|---|
// | `0x00..=0x0b` | the selector is itself a player index |
// | `0x0d` | the current player (`FUN_007a9d00`) |
// | `0x0e` | cursor slots with `relation[current][slot] == 0`, current excluded |
// | `0x0f` | cursor slots with `relation[current][slot] == 2`, current excluded |
// | `0x10` | cursor slots with `relation[current][slot] == 1`, current excluded |
// | `0x11` | every cursor slot, plus slots `8..=11` |
// | `0x12..=0x15` | cursor slots whose owner (`FUN_006c52e0 & 0xff`) equals `selector - 0x11` |
// | `0x16..=0x19` | none (returns `0`) |
// | `0x1a` | cursor slots that are not mutual class-`2` relations with the current player |
//
// ## Recovered tables (offsets from the xor-keyed game base)
//
// | offset | contents |
// |---|---|
// | `+0x2cf4` | first cached block, `0x150` dwords, mirrored to `0x00fdfe10` |
// | `+0x2d84` / `+0x2ed4` / `+0x2ff4` | trio A: first / second / third per-player tables |
// | `+0x2db4` / `+0x2f04` / `+0x3024` | trio B: the same three tables, `+0x30` from trio A |
// | `+0x2de4` / `+0x2e14` / `+0x2f34` / `+0x2f64` / `+0x3204` | five category tables |
// | `+0x5cf4` | second cached block, `0x2ac0` bytes, mirrored to `0x00fe0350` |
// | `+0x87b4` | per-unit table `[unit][player]`, stride `12`, trio B |
// | `+0xb274` | per-unit table `[unit][player]`, stride `12`, trio A |
// | `+0xe544` | `12 x 12` relation matrix, row stride `0x0c`, `[from][to]` |
// | `+0xe608` | dword compared by `0x00664210` |
// | `+0xe61c` / `+0xe61d` / `+0xe61e` | setup mode byte / byte argument / word argument |
//
// ## Static data used by this window
//
// | address | contents |
// |---|---|
// | `0x00fe58d0` | mode byte, cleared by `0x00664810`, set by `0x006656d0` |
// | `0x00fe58d1` | mode argument |
// | `0x00fe58d8` | eight `{ id: u32, value: u32 }` result records |
// | `0x00fdfe10` | cached player block (`0x540` bytes) |
// | `0x00fe0350` | cached per-unit block (`0x2ac0` bytes) |
//
// The slot cursor is the `FUN_0065b0c0` / `FUN_0065b0e0` pair.  Its exact walk
// is out of window, so every routine here takes the yielded index sequence as
// `W13A05World::cursor`; the recovered loops stop at the first index `>= 8`.
//
// Relation class `1` and class `2` are the two non-zero bytes of the `+0xe544`
// matrix.  The dispatch below uses class `2` for its enemy-style selector and
// class `1` for its ally-style selector, so those labels follow the selector,
// not an independently recovered enum.

/// Number of player slots addressed by the count tables.
pub const W13A05_SLOT_COUNT: usize = 12;
/// First index the slot cursor no longer yields for the filtered loops.
pub const W13A05_CURSOR_LIMIT: u32 = 8;
/// Upper bound (exclusive) of the per-unit table rows.
pub const W13A05_UNIT_LIMIT: u32 = 0xe9;
/// Rows of the per-unit tables (`0 ..= 0xe4`).
pub const W13A05_UNIT_ROWS: usize = 0xe5;
/// The four aggregate kind indices `0xe5..=0xe8`.  `0xe5` is the sum of the
/// two rows read by `0xe6`/`0xe7`, and `0xe8` is a third row.  The trigger
/// editor exposes these as its special unit entries; only the arithmetic is
/// recovered here.
pub const W13A05_KIND_SUM: u32 = 0xe5;
pub const W13A05_KIND_FIRST: u32 = 0xe6;
pub const W13A05_KIND_SECOND: u32 = 0xe7;
pub const W13A05_KIND_THIRD: u32 = 0xe8;
/// Relation classes stored in the `+0xe544` matrix.
pub const W13A05_REL_NONE: u8 = 0;
pub const W13A05_REL_ALLY: u8 = 1;
pub const W13A05_REL_ENEMY: u8 = 2;
/// Dwords copied from `+0x2cf4` into the cached player block.
pub const W13A05_CACHED_PLAYER_WORDS: usize = 0x150;
/// Dwords copied from `+0x5cf4` into the cached per-unit block.
pub const W13A05_CACHED_UNIT_WORDS: usize = 0x2ac0 / 4;
/// Base of the eight result records written by `0x00665dd0`.
pub const W13A05_RESULT_TABLE: usize = 0x00fe_58d8;
/// Sentinel id the setup routine writes into every result record.
pub const W13A05_RESULT_SENTINEL: u32 = 0xffff_ffff;

/// The `12 x 12` relation matrix at `+0xe544`, indexed `[from][to]`.
pub type W13A05Relation = [[u8; W13A05_SLOT_COUNT]; W13A05_SLOT_COUNT];

/// Everything the accumulate evaluators need besides their count tables.
#[derive(Clone, Copy, Debug)]
pub struct W13A05World<'a> {
    /// Index sequence produced by the slot cursor.
    pub cursor: &'a [u32],
    /// Player index returned by `FUN_007a9d00` for the running trigger.
    pub current: usize,
    /// Relation matrix at `+0xe544`.
    pub relation: &'a W13A05Relation,
    /// `FUN_006c52e0(slot) & 0xff` for every slot.
    pub owners: &'a [u8; W13A05_SLOT_COUNT],
    /// Value of the `DAT_01165f58` alternate provider switch; when present,
    /// `0x00664980` returns it for every per-unit row.
    pub override_value: Option<i32>,
}

impl<'a> W13A05World<'a> {
    /// Builds a world without the alternate provider switch.
    pub fn w13a05_new(
        cursor: &'a [u32],
        current: usize,
        relation: &'a W13A05Relation,
        owners: &'a [u8; W13A05_SLOT_COUNT],
    ) -> Self {
        Self {
            cursor,
            current,
            relation,
            owners,
            override_value: None,
        }
    }

    /// Enables the alternate provider (`DAT_01165f58 != 0`).
    pub fn w13a05_with_override(mut self, value: i32) -> Self {
        self.override_value = Some(value);
        self
    }

    /// Cursor indices below the loop bound, in order.
    fn w13a05_slots(&self) -> impl Iterator<Item = u32> + '_ {
        self.cursor
            .iter()
            .copied()
            .take_while(|slot| *slot < W13A05_CURSOR_LIMIT)
    }

    /// `relation[from][to]`, `0` when either index is out of range.
    fn w13a05_relation_at(&self, from: u32, to: u32) -> u8 {
        if from >= W13A05_SLOT_COUNT as u32 || to >= W13A05_SLOT_COUNT as u32 {
            return W13A05_REL_NONE;
        }
        self.relation[from as usize][to as usize]
    }

    /// Owner byte of a slot, `None` when the slot is out of range.
    fn w13a05_owner_at(&self, slot: u32) -> Option<u8> {
        self.owners.get(slot as usize).copied()
    }
}

/// Shared truth table of the two condition comparators.
fn w13a05_compare(target: u32, op: u8, current: u32) -> bool {
    match op {
        0 => target <= current,
        1 => current <= target,
        10 => target == current,
        _ => false,
    }
}

/// `0x00664210`: compares the condition operand `target` against the global
/// value at `+0xe608`.
///
/// ```text
/// current = *(u32 *)((base ^ key) + 0xe608);
/// switch (byte at parameter + 0xe) {
///   0  => target <= current,
///   1  => current <= target,
///   10 => current == target,
///   _  => false,
/// }
/// ```
pub fn w13a05_compare_global(target: u32, op: u8, current: u32) -> bool {
    w13a05_compare(target, op, current)
}

/// `0x00664260`: compares the condition operand `target` against a count read
/// for the player stored at `parameter + 4` (the read goes through the cached
/// provider `FUN_006651b0`).
///
/// The comparison itself is the same truth table as [`w13a05_compare_global`];
/// the two routines differ only in where the current value comes from.
pub fn w13a05_compare_player(target: u32, op: u8, current: u32) -> bool {
    w13a05_compare(target, op, current)
}

/// `0x00664610`: sums `visit` over the cursor slots below `8` and then over
/// slots `8..=11` unconditionally.
///
/// The shipped loop is `while (cursor < 8)` followed by a `do { } while
/// (next < 0xc)` that runs for `8, 9, 10, 11` exactly once each.
pub fn w13a05_sum_all_slots(world: &W13A05World<'_>, visit: &dyn Fn(u32) -> i32) -> i32 {
    let mut total = 0;
    for slot in world.w13a05_slots() {
        total += visit(slot);
    }
    for slot in W13A05_CURSOR_LIMIT..W13A05_SLOT_COUNT as u32 {
        total += visit(slot);
    }
    total
}

/// `0x00664700`: sums `visit` over the cursor slots that are not the current
/// player and whose `relation[current][slot]` is class `2`.
pub fn w13a05_sum_relation_enemy(world: &W13A05World<'_>, visit: &dyn Fn(u32) -> i32) -> i32 {
    w13a05_sum_relation_class(world, W13A05_REL_ENEMY, visit)
}

/// `0x00664810`: clears the accumulate mode byte at `0x00fe58d0`.
pub fn w13a05_clear_accumulate_mode(state: &mut W13A05Mode) {
    state.mode = 0;
}

/// `0x00664980`: the accumulate dispatcher over trio A.
///
/// ```text
/// switch (selector) {
///   0x0d => selector = FUN_007a9d00();        /* falls into the table read */
///   0x0e => sum relation class 0, recursion;  0x0f => class 2;
///   0x10 => class 1, recursion;               0x11 => every slot;
///   0x12..0x15 => owner == selector - 0x11;   0x16..0x19 => 0;
///   0x1a => not mutually class 2;             default => table read;
/// }
/// ```
///
/// The table read uses the per-player trio at `+0x2d84` / `+0x2ed4` /
/// `+0x2ff4` (`0xe5` = first + second, `0xe6` = first, `0xe7` = second,
/// `0xe8` = third) and the per-unit table at `+0xb274`, indexed
/// `player + unit * 12`.  When `DAT_01165f58` is non-zero every per-unit row
/// returns that global's value instead.
pub fn w13a05_accumulate_count(
    world: &W13A05World<'_>,
    counts: &W13A05TrioCounts,
    selector: u32,
    unit_type: u32,
) -> i32 {
    let over = world.override_value;
    let leaf = |player: u32, kind: u32| counts.w13a05_value(player, kind, over);
    w13a05_dispatch(world, selector, unit_type, &leaf, W13A05_SLOT_COUNT as u32)
}

/// `0x00664b60`: sums `visit` over the cursor slots that are not the current
/// player and whose `relation[current][slot]` is class `0`.
pub fn w13a05_sum_relation_none(world: &W13A05World<'_>, visit: &dyn Fn(u32) -> i32) -> i32 {
    w13a05_sum_relation_class(world, W13A05_REL_NONE, visit)
}

/// `0x00664be0`: [`w13a05_sum_relation_none`] with a target total.
///
/// The routine first reads the cursor; if the first index is already `>= 8`
/// it returns `0` without calling the visitor.  It stops as soon as the
/// running total equals `target`.  The visitor also receives `target`
/// (the fourth argument of the shipped callback).
pub fn w13a05_sum_relation_none_limited(
    world: &W13A05World<'_>,
    target: i32,
    visit: &dyn Fn(u32, i32) -> i32,
) -> i32 {
    match world.cursor.first() {
        None => return 0,
        Some(&first) if first >= W13A05_CURSOR_LIMIT => return 0,
        Some(_) => {}
    }
    let mut total = 0;
    for slot in world.w13a05_slots() {
        if slot == world.current as u32 {
            continue;
        }
        if world.w13a05_relation_at(world.current as u32, slot) == W13A05_REL_NONE {
            total += visit(slot, target);
            if total == target {
                return total;
            }
        }
    }
    total
}

/// `0x00664c70`: primes the cached count blocks once.
///
/// ```text
/// if (mode != 1) {
///     mode = 1;
///     copy 0x150 dwords from +0x2cf4 to 0x00fdfe10;
///     memcpy(0x00fe0350, +0x5cf4, 0x2ac0);
/// }
/// ```
///
/// Returns `true` when the copy ran.
pub fn w13a05_prime_cache(
    mode: &mut u8,
    cached_players: &mut [i32; W13A05_CACHED_PLAYER_WORDS],
    live_players: &[i32; W13A05_CACHED_PLAYER_WORDS],
    cached_units: &mut [i32; W13A05_CACHED_UNIT_WORDS],
    live_units: &[i32; W13A05_CACHED_UNIT_WORDS],
) -> bool {
    if *mode == 1 {
        return false;
    }
    *mode = 1;
    cached_players.copy_from_slice(live_players);
    cached_units.copy_from_slice(live_units);
    true
}

/// `0x00664ce0`: sums `visit` over the cursor slots whose owner byte equals
/// `selector - 0x11` (selectors `0x12..=0x15`).
pub fn w13a05_sum_owned_slots(
    world: &W13A05World<'_>,
    selector: u32,
    visit: &dyn Fn(u32) -> i32,
) -> i32 {
    if selector < 0x11 {
        return 0;
    }
    let wanted = selector - 0x11;
    let mut total = 0;
    for slot in world.w13a05_slots() {
        if world.w13a05_owner_at(slot).map(u32::from) == Some(wanted) {
            total += visit(slot);
        }
    }
    total
}

/// `0x00664d40`: [`w13a05_sum_owned_slots`] with a target total.
///
/// Like `0x00664be0` it bails out when the first cursor index is `>= 8` and
/// stops as soon as the running total reaches `target`; the visitor receives
/// `target` as its second argument.
pub fn w13a05_sum_owned_slots_limited(
    world: &W13A05World<'_>,
    selector: u32,
    target: i32,
    visit: &dyn Fn(u32, i32) -> i32,
) -> i32 {
    if selector < 0x11 {
        return 0;
    }
    match world.cursor.first() {
        None => return 0,
        Some(&first) if first >= W13A05_CURSOR_LIMIT => return 0,
        Some(_) => {}
    }
    let wanted = selector - 0x11;
    let mut total = 0;
    for slot in world.w13a05_slots() {
        if world.w13a05_owner_at(slot).map(u32::from) == Some(wanted) {
            total += visit(slot, target);
            if total == target {
                return total;
            }
        }
    }
    total
}

/// `0x00664db0`: the accumulate dispatcher over trio B.
///
/// Same selector dispatch as `0x00664980`, but the per-player tables sit at
/// `+0x2db4` / `+0x2f04` / `+0x3024` and the per-unit table at `+0x87b4`.
/// This routine has no `DAT_01165f58` alternate provider.
pub fn w13a05_accumulate_live_count(
    world: &W13A05World<'_>,
    counts: &W13A05TrioCounts,
    selector: u32,
    unit_type: u32,
) -> i32 {
    let leaf = |player: u32, kind: u32| counts.w13a05_value(player, kind, None);
    w13a05_dispatch(world, selector, unit_type, &leaf, W13A05_SLOT_COUNT as u32)
}

/// `0x00664f50`: sums `visit` over the cursor slots that are not the current
/// player and are **not** mutual class-`2` relations with it.
///
/// A slot is skipped only when `relation[slot][current] == 2` *and*
/// `relation[current][slot] == 2`; a one-way class-`2` entry still counts.
pub fn w13a05_sum_not_mutual_enemies(world: &W13A05World<'_>, visit: &dyn Fn(u32) -> i32) -> i32 {
    let current = world.current as u32;
    let mut total = 0;
    for slot in world.w13a05_slots() {
        if slot == current {
            continue;
        }
        if world.w13a05_relation_at(slot, current) != W13A05_REL_ENEMY
            || world.w13a05_relation_at(current, slot) != W13A05_REL_ENEMY
        {
            total += visit(slot);
        }
    }
    total
}

/// `0x006650a0`: sums `visit` over the cursor slots that are not the current
/// player and whose `relation[current][slot]` is class `1`.
pub fn w13a05_sum_relation_ally(world: &W13A05World<'_>, visit: &dyn Fn(u32) -> i32) -> i32 {
    w13a05_sum_relation_class(world, W13A05_REL_ALLY, visit)
}

/// `0x006651b0`: the accumulate dispatcher over the cached block.
///
/// The body is the same fourteen-way selector dispatch, but the per-player
/// read is delegated to `FUN_00748d40` and only accepts player indices below
/// `8`; every other fall-through selector returns `0`.  The reader is supplied
/// by the caller because its body is out of window.
pub fn w13a05_accumulate_cached_count(
    world: &W13A05World<'_>,
    selector: u32,
    leaf: &dyn Fn(u32) -> i32,
) -> i32 {
    let leaf = |player: u32, _kind: u32| leaf(player);
    w13a05_dispatch(world, selector, 0, &leaf, W13A05_CURSOR_LIMIT)
}

/// `0x006655e0`: sums over the selector's slot set the predicate
/// "`field` equals the slot index".
///
/// ```text
/// if (0xb < selector) return 0;
/// return *(u8 *)(parameter + 0x4c) == selector;
/// ```
///
/// `field` is the byte at `parameter + 0x4c`; because the recursive calls
/// forward the same parameter block, every slot is tested against it.
pub fn w13a05_accumulate_slot_flag(world: &W13A05World<'_>, selector: u32, field: u8) -> i32 {
    let leaf = |player: u32, _kind: u32| {
        if player < W13A05_SLOT_COUNT as u32 && u32::from(field) == player {
            1
        } else {
            0
        }
    };
    w13a05_dispatch(world, selector, 0, &leaf, W13A05_SLOT_COUNT as u32)
}

/// `0x006656d0`: switches the mode byte to `2` and records the argument.
///
/// Returns `false` (and changes nothing) when the mode is already `2` with the
/// same argument.  Otherwise the cached player block (`0x540` bytes at
/// `0x00fdfe10`) is zeroed.
pub fn w13a05_set_accumulate_mode(
    state: &mut W13A05Mode,
    argument: u8,
    cached_players: &mut [i32; W13A05_CACHED_PLAYER_WORDS],
) -> bool {
    if state.mode == 2 && state.argument == argument {
        return false;
    }
    state.mode = 2;
    state.argument = argument;
    cached_players.fill(0);
    true
}

/// `0x006657a0`: the accumulate dispatcher over the low pair of per-player
/// tables.
///
/// `which` (`parameter + 2`) selects `+0x00`, `+0x30`, or their sum; values
/// above `2` return `0`.
pub fn w13a05_accumulate_pair(
    world: &W13A05World<'_>,
    counts: &W13A05PairCounts,
    selector: u32,
    which: u32,
) -> i32 {
    let leaf = |player: u32, kind: u32| counts.w13a05_value(player, kind);
    w13a05_dispatch(world, selector, which, &leaf, W13A05_SLOT_COUNT as u32)
}

/// `0x006658e0`: the accumulate dispatcher over the five category tables.
///
/// `which` (`parameter + 2`) selects one of eight category combinations:
///
/// | `which` | value |
/// |---|---|
/// | `0` | all five tables |
/// | `1` | `+0x2de4` |
/// | `2` | `+0x2f34` |
/// | `3` | `+0x2f34 + +0x2de4` |
/// | `4` | `+0x2e14` |
/// | `5` | `+0x2f64` |
/// | `6` | `+0x2f64 + +0x2e14` |
/// | `7` | `+0x3204` |
/// | other | `0` |
pub fn w13a05_accumulate_category(
    world: &W13A05World<'_>,
    counts: &W13A05CategoryCounts,
    selector: u32,
    which: u32,
) -> i32 {
    let leaf = |player: u32, kind: u32| counts.w13a05_value(player, kind);
    w13a05_dispatch(world, selector, which, &leaf, W13A05_SLOT_COUNT as u32)
}

/// `0x00665ad0`: the accumulate dispatcher over the cached block, choosing
/// between its two layouts.
///
/// `which == 0` reads `0x00fdfe10` / `0x00fdff60` / `0x00fe00b0` with the
/// per-unit table at `0x00fe2e10`; any other value reads `0x00fdfe40` /
/// `0x00fdff90` / `0x00fe00e0` with the per-unit table at `0x00fe0350`.
/// Both trios use the `0xe5 = first + second`, `0xe6 = first`,
/// `0xe7 = second`, `0xe8 = third` mapping.
pub fn w13a05_accumulate_snapshot(
    world: &W13A05World<'_>,
    cached: &W13A05TrioCounts,
    live: &W13A05TrioCounts,
    selector: u32,
    unit_type: u32,
    which: u32,
) -> i32 {
    let chosen = if which == 0 { cached } else { live };
    let leaf = |player: u32, kind: u32| chosen.w13a05_value(player, kind, None);
    w13a05_dispatch(world, selector, unit_type, &leaf, W13A05_SLOT_COUNT as u32)
}

/// `0x00665dd0`: returns the base of the eight result records.
pub const fn w13a05_result_table_base() -> usize {
    W13A05_RESULT_TABLE
}

/// One result record written by the setup routine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13A05Result {
    /// Cached id; `0xffffffff` when the record is empty.
    pub id: u32,
    /// Cached value.
    pub value: u32,
}

/// The accumulate handler a setup mode installs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13A05Handler {
    /// `0x00665ad0` (cached block, two layouts).
    Snapshot,
    /// `0x006657a0` (low table pair).
    Pair,
    /// `0x00664db0` (trio B).
    Live,
    /// `0x006658e0` (five category tables).
    Category,
}

/// One `FUN_00665de0(handler, argument)` registration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W13A05Visit {
    /// Handler installed for the argument.
    pub handler: W13A05Handler,
    /// Argument forwarded to the handler.
    pub argument: u16,
}

/// The two handler tables the setup routine registers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W13A05Registration {
    /// Handler table used by the `0x11..=0x15` modes.
    ModeTable,
    /// Handler table used by the `0x21..=0x28` modes.
    VisitTable,
}

/// The plan `0x00665ec0` builds for one setup mode.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W13A05Setup {
    /// `FUN_00664c70` runs first.
    pub prime_cache: bool,
    /// `FUN_006656d0` runs with this argument.
    pub player_mode: Option<u8>,
    /// `FUN_00666230` installs this handler as the default.
    pub default_handler: Option<W13A05Handler>,
    /// `FUN_00665de0` registrations, in call order.
    pub visits: Vec<W13A05Visit>,
    /// Which handler table is registered last.
    pub registration: Option<W13A05Registration>,
}

/// `0x00665ec0`: resets the eight result records and classifies the setup mode.
///
/// The first action is always the same: every record at `0x00fe58d8` becomes
/// `{ 0xffffffff, 0 }`.  The mode byte at `+0xe61c` then selects one of the
/// plans in [`W13A05Setup`]; unlisted modes return `None` after the reset.
pub fn w13a05_setup(
    results: &mut [W13A05Result; 8],
    mode: u8,
    argument: u8,
    word: u16,
) -> Option<W13A05Setup> {
    for record in results.iter_mut() {
        *record = W13A05Result {
            id: W13A05_RESULT_SENTINEL,
            value: 0,
        };
    }
    let mut plan = W13A05Setup::default();
    match mode {
        0x11 => {
            plan.prime_cache = true;
            plan.default_handler = Some(W13A05Handler::Snapshot);
            plan.registration = Some(W13A05Registration::ModeTable);
        }
        0x12 => {
            plan.player_mode = Some(argument);
            plan.default_handler = Some(W13A05Handler::Snapshot);
            plan.registration = Some(W13A05Registration::ModeTable);
        }
        0x13 => {
            plan.default_handler = Some(W13A05Handler::Pair);
            plan.registration = Some(W13A05Registration::ModeTable);
        }
        0x14 => {
            plan.default_handler = Some(W13A05Handler::Live);
            plan.registration = Some(W13A05Registration::ModeTable);
        }
        0x15 => {
            plan.default_handler = Some(W13A05Handler::Category);
            plan.registration = Some(W13A05Registration::ModeTable);
        }
        0x21 => {
            plan.prime_cache = true;
            plan.visits.push(W13A05Visit {
                handler: W13A05Handler::Snapshot,
                argument: word,
            });
            plan.registration = Some(W13A05Registration::VisitTable);
        }
        0x22 => {
            plan.player_mode = Some(argument);
            plan.visits.push(W13A05Visit {
                handler: W13A05Handler::Snapshot,
                argument: word,
            });
            plan.registration = Some(W13A05Registration::VisitTable);
        }
        0x23 | 0x28 => {
            if mode == 0x23 && word != 2 {
                plan.visits.push(W13A05Visit {
                    handler: W13A05Handler::Pair,
                    argument: word,
                });
            } else {
                plan.visits.push(W13A05Visit {
                    handler: W13A05Handler::Pair,
                    argument: 0,
                });
                plan.visits.push(W13A05Visit {
                    handler: W13A05Handler::Pair,
                    argument: 1,
                });
            }
            plan.registration = Some(W13A05Registration::VisitTable);
        }
        0x24 => {
            plan.visits.push(W13A05Visit {
                handler: W13A05Handler::Live,
                argument: word,
            });
            plan.registration = Some(W13A05Registration::VisitTable);
        }
        0x25 => {
            match word {
                0 => {
                    for argument in [7, 1, 2, 4, 5] {
                        plan.visits.push(W13A05Visit {
                            handler: W13A05Handler::Category,
                            argument,
                        });
                    }
                }
                3 => {
                    for argument in [1, 2] {
                        plan.visits.push(W13A05Visit {
                            handler: W13A05Handler::Category,
                            argument,
                        });
                    }
                }
                6 => {
                    for argument in [4, 5] {
                        plan.visits.push(W13A05Visit {
                            handler: W13A05Handler::Category,
                            argument,
                        });
                    }
                }
                other => plan.visits.push(W13A05Visit {
                    handler: W13A05Handler::Category,
                    argument: other,
                }),
            }
            plan.registration = Some(W13A05Registration::VisitTable);
        }
        _ => return None,
    }
    Some(plan)
}

/// The mode state at `0x00fe58d0` / `0x00fe58d1`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13A05Mode {
    /// Mode byte.
    pub mode: u8,
    /// Mode argument.
    pub argument: u8,
}

/// Trio of per-player tables (`+0x2d84` / `+0x2ed4` / `+0x2ff4` and its
/// `+0x30`-shifted twin) plus the matching per-unit table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct W13A05TrioCounts {
    /// `0xe6` row.
    pub first: [i32; W13A05_SLOT_COUNT],
    /// `0xe7` row.
    pub second: [i32; W13A05_SLOT_COUNT],
    /// `0xe8` row.
    pub third: [i32; W13A05_SLOT_COUNT],
    /// `[unit_type][player]`, stride `12`, rows `0 ..= 0xe4`.
    pub per_unit: Vec<i32>,
}

impl W13A05TrioCounts {
    /// All-zero tables of the correct size.
    pub fn w13a05_zeroed() -> Self {
        Self {
            first: [0; W13A05_SLOT_COUNT],
            second: [0; W13A05_SLOT_COUNT],
            third: [0; W13A05_SLOT_COUNT],
            per_unit: vec![0; W13A05_UNIT_ROWS * W13A05_SLOT_COUNT],
        }
    }

    /// Reads the count for `player` and `unit_type`.
    ///
    /// `0xe5` is first + second, `0xe6` first, `0xe7` second, `0xe8` third;
    /// any other type below `0xe9` reads the per-unit table, where
    /// `override_value` (if any) takes precedence.  The shipped readers mask
    /// the kind with `0xffff` first.
    pub fn w13a05_value(&self, player: u32, unit_type: u32, override_value: Option<i32>) -> i32 {
        let unit_type = unit_type & 0xffff;
        if player >= W13A05_SLOT_COUNT as u32 || unit_type >= W13A05_UNIT_LIMIT {
            return 0;
        }
        let slot = player as usize;
        match unit_type {
            W13A05_KIND_SUM => self.first[slot] + self.second[slot],
            W13A05_KIND_FIRST => self.first[slot],
            W13A05_KIND_SECOND => self.second[slot],
            W13A05_KIND_THIRD => self.third[slot],
            _ => {
                if let Some(value) = override_value {
                    return value;
                }
                let index = unit_type as usize * W13A05_SLOT_COUNT + slot;
                self.per_unit.get(index).copied().unwrap_or(0)
            }
        }
    }
}

/// The low pair of per-player tables at `+0x00` and `+0x30`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct W13A05PairCounts {
    /// `+0x00` row.
    pub low: [i32; W13A05_SLOT_COUNT],
    /// `+0x30` row.
    pub high: [i32; W13A05_SLOT_COUNT],
}

impl W13A05PairCounts {
    /// `which` `0` reads `low`, `1` reads `high`, `2` reads their sum;
    /// every other value reads `0`.
    pub fn w13a05_value(&self, player: u32, which: u32) -> i32 {
        if player >= W13A05_SLOT_COUNT as u32 {
            return 0;
        }
        let slot = player as usize;
        match which & 0xffff {
            0 => self.low[slot],
            1 => self.high[slot],
            2 => self.low[slot] + self.high[slot],
            _ => 0,
        }
    }
}

/// The five category tables used by `0x006658e0`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct W13A05CategoryCounts {
    /// Tables read from `+0x2de4`, `+0x2e14`, `+0x2f34`, `+0x2f64`, `+0x3204`,
    /// in that order.
    pub tables: [[i32; W13A05_SLOT_COUNT]; 5],
}

impl W13A05CategoryCounts {
    /// Reads the eight recovered category combinations.
    pub fn w13a05_value(&self, player: u32, which: u32) -> i32 {
        if player >= W13A05_SLOT_COUNT as u32 {
            return 0;
        }
        let slot = player as usize;
        let table = &self.tables;
        match which & 0xffff {
            0 => table[0][slot] + table[1][slot] + table[2][slot] + table[3][slot] + table[4][slot],
            1 => table[0][slot],
            2 => table[2][slot],
            3 => table[2][slot] + table[0][slot],
            4 => table[1][slot],
            5 => table[3][slot],
            6 => table[3][slot] + table[1][slot],
            7 => table[4][slot],
            _ => 0,
        }
    }
}

/// Sums over the class-filtered selector set (selectors `0x0e`, `0x0f`,
/// `0x10`), skipping the current player.
fn w13a05_sum_relation_class(
    world: &W13A05World<'_>,
    class: u8,
    visit: &dyn Fn(u32) -> i32,
) -> i32 {
    let current = world.current as u32;
    let mut total = 0;
    for slot in world.w13a05_slots() {
        if slot == current {
            continue;
        }
        if world.w13a05_relation_at(current, slot) == class {
            total += visit(slot);
        }
    }
    total
}

/// The shared selector dispatch used by every accumulate routine in this
/// window.
///
/// `leaf` receives `(player, unit_type)` for a concrete player slot and
/// `player_limit` is the highest player index the leaf accepts (`12` for the
/// table readers, `8` for `0x006651b0`).
fn w13a05_dispatch(
    world: &W13A05World<'_>,
    selector: u32,
    unit_type: u32,
    leaf: &dyn Fn(u32, u32) -> i32,
    player_limit: u32,
) -> i32 {
    let recurse = |slot: u32| w13a05_dispatch(world, slot, unit_type, leaf, player_limit);
    match selector {
        0x0d => {
            let current = world.current as u32;
            if current < player_limit {
                leaf(current, unit_type)
            } else {
                0
            }
        }
        0x0e => w13a05_sum_relation_class(world, W13A05_REL_NONE, &recurse),
        0x0f => w13a05_sum_relation_class(world, W13A05_REL_ENEMY, &recurse),
        0x10 => w13a05_sum_relation_class(world, W13A05_REL_ALLY, &recurse),
        0x11 => w13a05_sum_all_slots(world, &recurse),
        0x12..=0x15 => w13a05_sum_owned_slots(world, selector, &recurse),
        0x16..=0x19 => 0,
        0x1a => w13a05_sum_not_mutual_enemies(world, &recurse),
        _ => {
            if selector < player_limit {
                leaf(selector, unit_type)
            } else {
                0
            }
        }
    }
}

#[cfg(test)]
mod tests_wave13_agent05 {
    use super::*;

    fn relation() -> W13A05Relation {
        // Current player 1: slot 2 is class 1, slot 3 is a one-way class 2,
        // slot 4 is a mutual class 2, slot 5 is class 0.
        let mut relation = [[W13A05_REL_NONE; W13A05_SLOT_COUNT]; W13A05_SLOT_COUNT];
        relation[1][2] = W13A05_REL_ALLY;
        relation[2][1] = W13A05_REL_ALLY;
        relation[1][3] = W13A05_REL_ENEMY;
        relation[1][4] = W13A05_REL_ENEMY;
        relation[4][1] = W13A05_REL_ENEMY;
        relation
    }

    const OWNERS: [u8; W13A05_SLOT_COUNT] = [0, 1, 2, 3, 1, 2, 3, 0, 0, 1, 2, 3];
    const CURSOR: [u32; 8] = [0, 1, 2, 3, 4, 5, 6, 7];

    fn world<'a>(relation: &'a W13A05Relation, owners: &'a [u8; 12]) -> W13A05World<'a> {
        W13A05World::w13a05_new(&CURSOR, 1, relation, owners)
    }

    fn trio() -> W13A05TrioCounts {
        let mut counts = W13A05TrioCounts::w13a05_zeroed();
        for player in 0..W13A05_SLOT_COUNT {
            counts.first[player] = 100 + player as i32;
            counts.second[player] = 200 + player as i32;
            counts.third[player] = 300 + player as i32;
        }
        for unit in 0..W13A05_UNIT_ROWS {
            for player in 0..W13A05_SLOT_COUNT {
                counts.per_unit[unit * W13A05_SLOT_COUNT + player] =
                    player as i32 * 1000 + unit as i32;
            }
        }
        counts
    }

    fn pair() -> W13A05PairCounts {
        let mut counts = W13A05PairCounts {
            low: [0; W13A05_SLOT_COUNT],
            high: [0; W13A05_SLOT_COUNT],
        };
        for player in 0..W13A05_SLOT_COUNT {
            counts.low[player] = player as i32;
            counts.high[player] = 1000 + player as i32;
        }
        counts
    }

    fn categories() -> W13A05CategoryCounts {
        let mut counts = W13A05CategoryCounts {
            tables: [[0; W13A05_SLOT_COUNT]; 5],
        };
        for player in 0..W13A05_SLOT_COUNT {
            let base = player as i32;
            counts.tables[0][player] = base;
            counts.tables[1][player] = 10 + base;
            counts.tables[2][player] = 20 + base;
            counts.tables[3][player] = 30 + base;
            counts.tables[4][player] = 40 + base;
        }
        counts
    }

    #[test]
    fn compare_operators_and_boundaries() {
        // 0x00664210 / 0x00664260: op 0 is `target <= current`.
        assert!(w13a05_compare_global(5, 0, 5));
        assert!(w13a05_compare_global(4, 0, 5));
        assert!(!w13a05_compare_global(6, 0, 5));
        assert!(w13a05_compare_player(6, 1, 5));
        assert!(w13a05_compare_player(5, 1, 5));
        assert!(!w13a05_compare_player(4, 1, 5));
        assert!(w13a05_compare_global(u32::MAX, 10, u32::MAX));
        assert!(!w13a05_compare_global(0, 10, u32::MAX));
        for op in [2u8, 9, 11, 0xff] {
            assert!(!w13a05_compare_global(1, op, 1));
        }
    }

    #[test]
    fn sum_all_slots_covers_cursor_then_eight_to_eleven() {
        // 0x00664610
        let relation = relation();
        let world = world(&relation, &OWNERS);
        let visited = std::cell::RefCell::new(Vec::new());
        let total = w13a05_sum_all_slots(&world, &|slot| {
            visited.borrow_mut().push(slot);
            slot as i32
        });
        assert_eq!(visited.into_inner(), (0..12).collect::<Vec<u32>>());
        assert_eq!(total, 66);
    }

    #[test]
    fn relation_sums_filter_and_skip_current() {
        // 0x00664b60 (class 0), 0x00664700 (class 2), 0x006650a0 (class 1)
        let relation = relation();
        let world = world(&relation, &OWNERS);
        let collect = |slot: u32| slot as i32;

        let none: Vec<u32> = world
            .w13a05_slots()
            .filter(|&slot| slot != 1 && relation[1][slot as usize] == 0)
            .collect();
        assert_eq!(none, vec![0, 5, 6, 7]);
        assert_eq!(w13a05_sum_relation_none(&world, &collect), 18);
        assert_eq!(w13a05_sum_relation_enemy(&world, &collect), 7);
        assert_eq!(w13a05_sum_relation_ally(&world, &collect), 2);
    }

    #[test]
    fn limited_relation_sum_bails_and_stops_at_target() {
        // 0x00664be0
        let relation = relation();
        let world = world(&relation, &OWNERS);
        assert_eq!(
            w13a05_sum_relation_none_limited(&world, 100, &|slot, target| {
                assert_eq!(target, 100);
                slot as i32
            }),
            18
        );
        // Stops as soon as the running total reaches the target.
        assert_eq!(
            w13a05_sum_relation_none_limited(&world, 5, &|slot, _| slot as i32),
            5
        );
        // A cursor whose first index is already out of range returns 0.
        let late = [8u32, 0, 1];
        let late_world = W13A05World::w13a05_new(&late, 1, &relation, &OWNERS);
        assert_eq!(
            w13a05_sum_relation_none_limited(&late_world, 10, &|_, _| 1),
            0
        );
    }

    #[test]
    fn owned_slot_sums() {
        // 0x00664ce0 / 0x00664d40
        let relation = relation();
        let world = world(&relation, &OWNERS);
        // selector 0x12 wants owner 1: slots 1 and 4 (the owner loop never
        // walks slots 8..11).
        assert_eq!(w13a05_sum_owned_slots(&world, 0x12, &|slot| slot as i32), 5);
        // selector 0x13 wants owner 2: slots 2 and 5.
        assert_eq!(w13a05_sum_owned_slots(&world, 0x13, &|slot| slot as i32), 7);
        assert_eq!(w13a05_sum_owned_slots(&world, 0x10, &|_| 1), 0);
        assert_eq!(
            w13a05_sum_owned_slots_limited(&world, 0x12, 99, &|slot, _| slot as i32),
            5
        );
        assert_eq!(
            w13a05_sum_owned_slots_limited(&world, 0x12, 1, &|slot, _| slot as i32),
            1
        );
        let late = [9u32, 0];
        let late_world = W13A05World::w13a05_new(&late, 1, &relation, &OWNERS);
        assert_eq!(
            w13a05_sum_owned_slots_limited(&late_world, 0x12, 99, &|_, _| 1),
            0
        );
    }

    #[test]
    fn not_mutual_enemies_only_drops_two_way_hostility() {
        // 0x00664f50
        let relation = relation();
        let world = world(&relation, &OWNERS);
        let visited = std::cell::RefCell::new(Vec::new());
        let total = w13a05_sum_not_mutual_enemies(&world, &|slot| {
            visited.borrow_mut().push(slot);
            1
        });
        assert_eq!(visited.into_inner(), vec![0, 2, 3, 5, 6, 7]);
        assert_eq!(total, 6);
    }

    #[test]
    fn trio_value_maps_kinds_and_units() {
        let counts = trio();
        assert_eq!(counts.w13a05_value(2, 0xe5, None), 304);
        assert_eq!(counts.w13a05_value(2, 0xe6, None), 102);
        assert_eq!(counts.w13a05_value(2, 0xe7, None), 202);
        assert_eq!(counts.w13a05_value(2, 0xe8, None), 302);
        assert_eq!(counts.w13a05_value(2, 5, None), 2005);
        assert_eq!(counts.w13a05_value(2, 0xe4, None), 2228);
        assert_eq!(counts.w13a05_value(12, 5, None), 0);
        assert_eq!(counts.w13a05_value(2, 0xe9, None), 0);
        assert_eq!(counts.w13a05_value(2, 5, Some(-7)), -7);
        // The shipped readers mask the kind with 0xffff.
        assert_eq!(counts.w13a05_value(2, 0x1_0005, None), 2005);
        assert_eq!(counts.w13a05_value(2, 0x1_00e6, None), 102);
        // The kind rows ignore the alternate provider.
        assert_eq!(counts.w13a05_value(2, 0xe6, Some(-7)), 102);
    }

    #[test]
    fn dispatch_over_trio_a() {
        // 0x00664980
        let relation = relation();
        let world = world(&relation, &OWNERS);
        let counts = trio();
        let value = |selector: u32| w13a05_accumulate_count(&world, &counts, selector, 5);

        assert_eq!(value(0x0d), 1005);
        assert_eq!(value(0x0e), 18020);
        assert_eq!(value(0x0f), 3005 + 4005);
        assert_eq!(value(0x10), 2005);
        assert_eq!(value(0x11), 66060);
        assert_eq!(value(0x12), 1005 + 4005);
        for selector in 0x16..=0x19 {
            assert_eq!(value(selector), 0);
        }
        assert_eq!(value(0x1a), 23030);
        assert_eq!(value(0x0c), 0);
        assert_eq!(value(0x1b), 0);

        // `DAT_01165f58 != 0` replaces every per-unit row.
        let overridden = world.w13a05_with_override(-5);
        assert_eq!(w13a05_accumulate_count(&overridden, &counts, 0x0d, 5), -5);
        assert_eq!(
            w13a05_accumulate_count(&overridden, &counts, 0x0d, 0xe6),
            101
        );
    }

    #[test]
    fn dispatch_over_trio_b_uses_its_own_override_free_tables() {
        // 0x00664db0
        let relation = relation();
        let mut world = world(&relation, &OWNERS);
        world.override_value = Some(999);
        let mut counts = trio();
        counts.per_unit[5 * 12 + 1] = 42;
        assert_eq!(w13a05_accumulate_live_count(&world, &counts, 0x0d, 5), 42);
        assert_eq!(w13a05_accumulate_live_count(&world, &counts, 0xe6, 5), 0);
        assert_eq!(
            w13a05_accumulate_live_count(&world, &counts, 0x11, 0xe5),
            3732
        );
    }

    #[test]
    fn pair_tables_and_dispatch() {
        // 0x006657a0
        let relation = relation();
        let world = world(&relation, &OWNERS);
        let counts = pair();
        assert_eq!(counts.w13a05_value(3, 0), 3);
        assert_eq!(counts.w13a05_value(3, 1), 1003);
        assert_eq!(counts.w13a05_value(3, 2), 1006);
        assert_eq!(counts.w13a05_value(3, 3), 0);
        assert_eq!(counts.w13a05_value(3, 0x1_0002), 1006);
        // 0x11 sums every slot; which 2 is low + high.
        assert_eq!(
            w13a05_accumulate_pair(&world, &counts, 0x11, 2),
            12000 + 132
        );
        // 0x0f is slots 3 and 4.
        assert_eq!(w13a05_accumulate_pair(&world, &counts, 0x0f, 1), 2007);
        assert_eq!(w13a05_accumulate_pair(&world, &counts, 0x0d, 0), 1);
    }

    #[test]
    fn category_tables_match_the_eight_combinations() {
        // 0x006658e0
        let relation = relation();
        let world = world(&relation, &OWNERS);
        let counts = categories();
        for (which, expected) in [
            (0, 100),
            (1, 0),
            (2, 20),
            (3, 20),
            (4, 10),
            (5, 30),
            (6, 40),
            (7, 40),
        ] {
            assert_eq!(counts.w13a05_value(0, which), expected, "which {which}");
        }
        assert_eq!(counts.w13a05_value(0, 8), 0);
        assert_eq!(counts.w13a05_value(0, 0x1_0000), 100);
        assert_eq!(counts.w13a05_value(12, 0), 0);
        // Selector 0x0d on player 1, category 0 sums all five tables.
        assert_eq!(w13a05_accumulate_category(&world, &counts, 0x0d, 0), 105);
        // 0x16..=0x19 return zero even for a valid category.
        assert_eq!(w13a05_accumulate_category(&world, &counts, 0x17, 0), 0);
    }

    #[test]
    fn snapshot_dispatch_reads_the_chosen_trio() {
        // 0x00665ad0
        let relation = relation();
        let world = world(&relation, &OWNERS);
        let mut cached = trio();
        cached.first = [1; W13A05_SLOT_COUNT];
        cached.second = [2; W13A05_SLOT_COUNT];
        cached.third = [3; W13A05_SLOT_COUNT];
        let mut live = trio();
        live.first = [10; W13A05_SLOT_COUNT];
        live.second = [20; W13A05_SLOT_COUNT];
        live.third = [30; W13A05_SLOT_COUNT];

        // `which == 0` selects the cached layout, anything else the live one.
        assert_eq!(
            w13a05_accumulate_snapshot(&world, &cached, &live, 0x0d, 0xe6, 0),
            1
        );
        assert_eq!(
            w13a05_accumulate_snapshot(&world, &cached, &live, 0x0d, 0xe6, 1),
            10
        );
        assert_eq!(
            w13a05_accumulate_snapshot(&world, &cached, &live, 0x0d, 0xe5, 0),
            3
        );
        assert_eq!(
            w13a05_accumulate_snapshot(&world, &cached, &live, 0x0d, 0xe8, 1),
            30
        );
        assert_eq!(
            w13a05_accumulate_snapshot(&world, &cached, &live, 0x0d, 0xe9, 0),
            0
        );
    }

    #[test]
    fn slot_flag_selector_counts_membership() {
        // 0x006655e0
        let relation = relation();
        let world = world(&relation, &OWNERS);
        // Field 7 lies in the class-0 set {0,5,6,7}; field 3 does not.
        assert_eq!(w13a05_accumulate_slot_flag(&world, 0x0e, 3), 0);
        assert_eq!(w13a05_accumulate_slot_flag(&world, 0x0e, 5), 1);
        assert_eq!(w13a05_accumulate_slot_flag(&world, 0x0e, 7), 1);
        assert_eq!(w13a05_accumulate_slot_flag(&world, 0x0e, 1), 0);
        assert_eq!(w13a05_accumulate_slot_flag(&world, 0x11, 11), 1);
        assert_eq!(w13a05_accumulate_slot_flag(&world, 0x11, 12), 0);
        assert_eq!(w13a05_accumulate_slot_flag(&world, 0x0c, 0), 0);
    }

    #[test]
    fn cached_dispatch_limits_players_to_eight() {
        // 0x006651b0
        let relation = relation();
        let mut world = world(&relation, &OWNERS);
        let leaf = |player: u32| player as i32 * 10;
        assert_eq!(w13a05_accumulate_cached_count(&world, 0x0d, &leaf), 10);
        // The 0x11 set also walks slots 8..11, but this dispatcher's reader
        // only accepts players below 8, so those slots contribute zero.
        assert_eq!(w13a05_accumulate_cached_count(&world, 0x11, &leaf), 280);
        assert_eq!(w13a05_accumulate_cached_count(&world, 0x0f, &leaf), 70);
        assert_eq!(w13a05_accumulate_cached_count(&world, 0x07, &leaf), 70);
        // Selectors 8..=12 fall outside the reader's eight-player bound.
        assert_eq!(w13a05_accumulate_cached_count(&world, 0x0b, &leaf), 0);
        assert_eq!(w13a05_accumulate_cached_count(&world, 0x0c, &leaf), 0);
        world.current = 9;
        assert_eq!(w13a05_accumulate_cached_count(&world, 0x0d, &leaf), 0);
    }

    #[test]
    fn prime_cache_runs_once() {
        // 0x00664c70
        let mut mode = 0u8;
        let mut cached_players = [0i32; W13A05_CACHED_PLAYER_WORDS];
        let mut cached_units = [0i32; W13A05_CACHED_UNIT_WORDS];
        let mut live_players = [0i32; W13A05_CACHED_PLAYER_WORDS];
        let mut live_units = [0i32; W13A05_CACHED_UNIT_WORDS];
        live_players[0] = 11;
        live_players[W13A05_CACHED_PLAYER_WORDS - 1] = 12;
        live_units[0] = 13;
        live_units[W13A05_CACHED_UNIT_WORDS - 1] = 14;

        assert!(w13a05_prime_cache(
            &mut mode,
            &mut cached_players,
            &live_players,
            &mut cached_units,
            &live_units
        ));
        assert_eq!(mode, 1);
        assert_eq!(cached_players, live_players);
        assert_eq!(cached_units, live_units);

        live_players[0] = 99;
        assert!(!w13a05_prime_cache(
            &mut mode,
            &mut cached_players,
            &live_players,
            &mut cached_units,
            &live_units
        ));
        assert_eq!(cached_players[0], 11);
    }

    #[test]
    fn mode_byte_setters() {
        // 0x00664810 and 0x006656d0
        let mut state = W13A05Mode {
            mode: 2,
            argument: 4,
        };
        w13a05_clear_accumulate_mode(&mut state);
        assert_eq!(state.mode, 0);
        assert_eq!(state.argument, 4);

        let mut cached = [7i32; W13A05_CACHED_PLAYER_WORDS];
        assert!(w13a05_set_accumulate_mode(&mut state, 5, &mut cached));
        assert_eq!(state.mode, 2);
        assert_eq!(state.argument, 5);
        assert!(cached.iter().all(|&word| word == 0));

        cached[0] = 1;
        assert!(!w13a05_set_accumulate_mode(&mut state, 5, &mut cached));
        assert_eq!(cached[0], 1);
        assert!(w13a05_set_accumulate_mode(&mut state, 6, &mut cached));
    }

    #[test]
    fn result_table_address_is_recovered() {
        // 0x00665dd0
        assert_eq!(w13a05_result_table_base(), 0x00fe_58d8);
    }

    #[test]
    fn setup_resets_records_before_classifying() {
        // 0x00665ec0
        let mut results = [W13A05Result { id: 1, value: 2 }; 8];
        let plan = w13a05_setup(&mut results, 0x11, 0, 0).expect("mode 0x11 is known");
        assert!(
            results
                .iter()
                .all(|record| record.id == W13A05_RESULT_SENTINEL && record.value == 0)
        );
        assert!(plan.prime_cache);
        assert_eq!(plan.default_handler, Some(W13A05Handler::Snapshot));
        assert_eq!(plan.registration, Some(W13A05Registration::ModeTable));
        assert!(plan.visits.is_empty());

        let unknown = w13a05_setup(&mut results, 0x99, 7, 7);
        assert!(unknown.is_none());
        assert_eq!(results[0].id, W13A05_RESULT_SENTINEL);
    }

    #[test]
    fn setup_modes_register_the_recovered_handlers() {
        // 0x00665ec0
        let mut results = [W13A05Result { id: 0, value: 0 }; 8];

        let plan = w13a05_setup(&mut results, 0x12, 3, 0).unwrap();
        assert!(!plan.prime_cache);
        assert_eq!(plan.player_mode, Some(3));
        assert_eq!(plan.default_handler, Some(W13A05Handler::Snapshot));
        assert_eq!(plan.registration, Some(W13A05Registration::ModeTable));

        for (mode, handler) in [
            (0x13u8, W13A05Handler::Pair),
            (0x14, W13A05Handler::Live),
            (0x15, W13A05Handler::Category),
        ] {
            let plan = w13a05_setup(&mut results, mode, 0, 0).unwrap();
            assert_eq!(plan.default_handler, Some(handler), "mode {mode:#x}");
            assert_eq!(plan.registration, Some(W13A05Registration::ModeTable));
            assert!(!plan.prime_cache);
        }

        let plan = w13a05_setup(&mut results, 0x21, 0, 4).unwrap();
        assert!(plan.prime_cache);
        assert_eq!(
            plan.visits,
            vec![W13A05Visit {
                handler: W13A05Handler::Snapshot,
                argument: 4
            }]
        );
        assert_eq!(plan.registration, Some(W13A05Registration::VisitTable));

        let plan = w13a05_setup(&mut results, 0x22, 8, 4).unwrap();
        assert_eq!(plan.player_mode, Some(8));
        assert_eq!(plan.visits.len(), 1);

        // 0x23: word 2 expands into the pair 0 + 1, other words stay singular.
        let plan = w13a05_setup(&mut results, 0x23, 0, 2).unwrap();
        assert_eq!(
            plan.visits
                .iter()
                .map(|visit| visit.argument)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        let plan = w13a05_setup(&mut results, 0x23, 0, 5).unwrap();
        assert_eq!(plan.visits.len(), 1);

        let plan = w13a05_setup(&mut results, 0x24, 0, 6).unwrap();
        assert_eq!(plan.visits[0].handler, W13A05Handler::Live);

        // 0x25: categories 0/3/6 expand, everything else is passed through.
        let plan = w13a05_setup(&mut results, 0x25, 0, 0).unwrap();
        assert_eq!(
            plan.visits
                .iter()
                .map(|visit| visit.argument)
                .collect::<Vec<_>>(),
            vec![7, 1, 2, 4, 5]
        );
        let plan = w13a05_setup(&mut results, 0x25, 0, 3).unwrap();
        assert_eq!(
            plan.visits
                .iter()
                .map(|visit| visit.argument)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
        let plan = w13a05_setup(&mut results, 0x25, 0, 6).unwrap();
        assert_eq!(
            plan.visits
                .iter()
                .map(|visit| visit.argument)
                .collect::<Vec<_>>(),
            vec![4, 5]
        );
        let plan = w13a05_setup(&mut results, 0x25, 0, 7).unwrap();
        assert_eq!(plan.visits[0].argument, 7);
        assert_eq!(plan.registration, Some(W13A05Registration::VisitTable));

        // 0x28 registers both halves of the low pair.
        let plan = w13a05_setup(&mut results, 0x28, 0, 0).unwrap();
        assert_eq!(
            plan.visits,
            vec![
                W13A05Visit {
                    handler: W13A05Handler::Pair,
                    argument: 0
                },
                W13A05Visit {
                    handler: W13A05Handler::Pair,
                    argument: 1
                }
            ]
        );
    }
}
