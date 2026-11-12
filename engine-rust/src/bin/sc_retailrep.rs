//! Play a retail Remastered `.rep` through the engine and report where it
//! first diverges from the recorded game.
//!
//! ```text
//! sc_retailrep REPLAY.rep --install DIR [--dump-units] [--report]
//! ```
//!
//! A retail replay stores the human players' commands only (no state hashes),
//! so "divergence" means: a command that cannot be executed in the engine
//! (unknown unit, wrong owner, unaffordable build, unreachable placement) or
//! a game that does not end where the header says it does.
//!
//! Unit tags: the retail replay names units by a 16-bit tag.  Measured on a
//! real replay: `tag = 11595 - creation index` for the units loaded from the
//! map and the melee starting forces (see `docs/FIDELITY.md`).

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use galaxycraft::ai::SimpleAi;
use galaxycraft::map::{Scenario, TileTables};
use galaxycraft::retail::RetailInstall;
use galaxycraft::retail_replay::RetailReplay;
use galaxycraft::sim::{Order, Race, SimError, World};

const TAG_BASE: i64 = 11595;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut path = None;
    let mut install = std::env::var("SC_INSTALL").ok();
    let mut dump = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--install" => {
                i += 1;
                install = args.get(i).cloned();
            }
            "--dump-units" => dump = true,
            other => path = Some(other.to_string()),
        }
        i += 1;
    }
    let path = path.expect("usage: sc_retailrep REPLAY.rep --install DIR");
    let bytes = std::fs::read(&path).expect("read replay");
    let rep = RetailReplay::parse(&bytes).expect("parse replay");
    let header = rep.header.clone().expect("replay header");
    let retail = RetailInstall::open(install.as_deref().expect("--install or SC_INSTALL"))
        .expect("retail install");
    let data = Arc::new(retail.game_data().expect("retail tables"));
    // Melee start locations are assigned at game start (random in this
    // replay), so the map's own owners may not match the game.  Give the
    // human (player 0, the one in the action log) the start location nearest
    // to its first positional command, and the other one to the rest.
    let chk = assign_start_locations(&rep);
    let scenario = Scenario::parse(&chk).expect("embedded CHK");
    let era = galaxycraft::render::era_name(scenario.era);
    let tmp = std::env::temp_dir().join("sc_retailrep");
    std::fs::create_dir_all(&tmp).ok();
    for ext in ["vf4", "cv5"] {
        let bytes = retail
            .read(&format!("TileSet/{era}.{ext}"))
            .unwrap_or_else(|| panic!("missing tileset {era}.{ext}"));
        std::fs::write(tmp.join(format!("t.{ext}")), bytes).expect("write tileset");
    }
    let tiles = TileTables::read_vf4(&tmp.join("t.vf4")).expect("tileset");
    // Player ids in the action log are game player ids; the header lists the
    // roster by slot.  Human race first (commands use player 0), then the rest.
    let races: Vec<Race> = {
        let mut r: Vec<(bool, Race)> = header
            .slots
            .iter()
            .map(|s| {
                (
                    s.kind == 2,
                    match s.race {
                        0 => Race::Zerg,
                        1 => Race::Terran,
                        _ => Race::Protoss,
                    },
                )
            })
            .collect();
        r.sort_by_key(|(human, _)| !*human);
        r.into_iter().map(|(_, race)| race).collect()
    };
    println!(
        "map {} {}x{} frames {} races {:?}",
        header.map_name, header.map_width, header.map_height, header.frames, races
    );
    let mut world = World::from_scenario(data, &scenario, &tiles.data(), &races);
    let static_ids = world
        .units
        .iter()
        .find(|u| u.owner == 1)
        .map_or(world.units.len() as u32, |u| u.id - 1);
    let tags = TagMap::new(&chk, static_ids);
    if dump {
        for u in world.units.iter().take(120) {
            println!(
                "id {:>3} tag {:>5} owner {} type {:>3} {:<24} at ({},{})",
                u.id,
                tags.tag_of(u.id),
                u.owner,
                u.type_id,
                world.data.unit_name(u.type_id),
                u.x / 256,
                u.y / 256
            );
        }
    }
    let mut ais: Vec<SimpleAi> = (1..world.players.len() as u8).map(SimpleAi::new).collect();
    let mut st = Driver::default();
    let mut next_block = 0usize;
    let last = rep.actions.last().map(|b| b.frame).unwrap_or(0);
    while (world.tick as u32) <= last.max(header.frames) && world.winner.is_none() {
        let now = world.tick as u32;
        while next_block < rep.actions.len() && rep.actions[next_block].frame <= now {
            let b = &rep.actions[next_block];
            st.apply_block(&mut world, &tags, b.frame, &b.data);
            next_block += 1;
        }
        if std::env::var("RETAIL_TRACE").is_ok_and(|v| v.split(',').any(|f| f.parse() == Ok(now))) {
            println!(
                "  t={now} minerals {} gas {} mined {}",
                world.players[0].minerals, world.players[0].gas, world.players[0].minerals_gathered
            );
            for u in world.units.iter().filter(|u| {
                u.alive
                    && u.owner == 0
                    && !u.is_resource()
                    && u.type_id != galaxycraft::ids::unit::VESPENE_GEYSER
            }) {
                println!(
                    "  t={now} #{} {} at ({},{}) order {:?} completed {} left {}",
                    u.id,
                    world.data.unit_name(u.type_id),
                    u.x / 256,
                    u.y / 256,
                    u.order,
                    u.completed,
                    u.build_remaining
                );
            }
        }
        for ai in &mut ais {
            let target = world
                .units
                .iter()
                .find(|u| u.alive && u.owner == 0 && u.completed)
                .map(|u| (u.x / 256, u.y / 256));
            ai.update(&mut world, target);
        }
        world.step();
        if now.is_multiple_of(2400) {
            println!(
                "frame {now:>6}: minerals {:?} gas {:?} units p0={} p1={}",
                world.players.iter().map(|p| p.minerals).collect::<Vec<_>>(),
                world.players.iter().map(|p| p.gas).collect::<Vec<_>>(),
                world.unit_ids(0).len(),
                world.unit_ids(1).len()
            );
        }
    }
    println!(
        "\nreplay ends at frame {} (header says {}); engine tick {} winner {:?}",
        last, header.frames, world.tick, world.winner
    );
    st.report();
}

/// Retail unit tags: `tag = TAG_BASE - creation index`, where the creation
/// index counts every record of the map's UNIT section (start locations
/// included, although the engine does not instantiate them) and then the
/// melee starting units in player order.
struct TagMap {
    /// Retail index of engine unit ids `1..=map_units`.
    map_idx: Vec<usize>,
    start_locations: usize,
    idx_to_id: HashMap<usize, u32>,
}

impl TagMap {
    /// `static_ids`: engine ids `1..=static_ids` exist at game start on the
    /// human side (map units and the human's starting force); only those have
    /// derivable tags.
    fn new(chk: &[u8], static_ids: u32) -> Self {
        let mut map_idx = Vec::new();
        let mut start_locations = 0;
        let mut o = 0usize;
        while o + 8 <= chk.len() {
            let name = &chk[o..o + 4];
            let size =
                u32::from_le_bytes([chk[o + 4], chk[o + 5], chk[o + 6], chk[o + 7]]) as usize;
            let end = (o + 8 + size).min(chk.len());
            if name == b"UNIT" {
                map_idx.clear();
                start_locations = 0;
                for (i, r) in chk[o + 8..end].as_chunks::<36>().0.iter().enumerate() {
                    if u16::from_le_bytes([r[8], r[9]]) == 214 {
                        start_locations += 1;
                    } else {
                        map_idx.push(i);
                    }
                }
            }
            o = end;
        }
        let mut t = Self {
            map_idx,
            start_locations,
            idx_to_id: HashMap::new(),
        };
        for id in 1..=static_ids {
            t.idx_to_id.insert(t.index_of(id), id);
        }
        t
    }

    fn index_of(&self, id: u32) -> usize {
        match self.map_idx.get(id as usize - 1) {
            Some(i) => *i,
            None => id as usize - 1 + self.start_locations,
        }
    }

    fn tag_of(&self, id: u32) -> i64 {
        TAG_BASE - self.index_of(id) as i64
    }

    /// Engine unit id for a retail tag, or why it cannot be resolved.
    fn id_of(&self, tag: u16) -> Result<u32, String> {
        if tag >> 11 != (TAG_BASE as u16) >> 11 {
            return Err(format!(
                "tag {tag:#06x} has generation {} (a recycled slot; the engine does not recycle)",
                tag >> 11
            ));
        }
        let idx = (TAG_BASE - tag as i64) as usize;
        self.idx_to_id
            .get(&idx)
            .copied()
            .ok_or_else(|| format!("tag {tag:#06x} (creation index {idx}) is not an engine unit"))
    }
}

/// One selected unit: bound to an engine unit, or a retail tag that is not
/// bound yet (a unit created after the start, whose tag cannot be derived:
/// retail allocates tags from one global slot list the Zerg AI also uses).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Sel {
    Unit(u32),
    Tag(u16),
}

/// What kind of engine unit a pending tag must be bound to, taken from the
/// command that uses it.
#[derive(Clone, Copy)]
enum Class {
    /// A unit that can produce `unit_type`.
    Producer(u16),
    Worker,
    Military,
    /// Any unit that can move (workers included).
    Mobile,
    AnyBuilding,
}

#[derive(Default)]
struct Driver {
    selection: HashMap<u8, Vec<Sel>>,
    groups: HashMap<(u8, u8), Vec<Sel>>,
    /// Tags bound by context: retail tag -> engine unit id.
    bound: HashMap<u16, u32>,
    applied: BTreeMap<u8, usize>,
    unsupported: BTreeMap<u8, usize>,
    failures: Vec<(u32, u8, String)>,
    binds: usize,
    /// Minerals the driver had to add so a recorded build could start: the
    /// engine economy was behind the recorded game by this much.
    deficit: i64,
}

fn u16_at(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}

impl Driver {
    fn fail(&mut self, frame: u32, kind: u8, msg: String) {
        self.failures.push((frame, kind, msg));
    }

    fn ok(&mut self, kind: u8) {
        *self.applied.entry(kind).or_default() += 1;
    }

    fn apply_block(&mut self, w: &mut World, tags: &TagMap, frame: u32, mut d: &[u8]) {
        while d.len() >= 2 {
            let (player, kind) = (d[0], d[1]);
            let body = &d[2..];
            let size = match kind {
                0x09..=0x0b => 1 + 2 * *body.first().unwrap_or(&0) as usize,
                0x63..=0x65 => 1 + 4 * *body.first().unwrap_or(&0) as usize,
                0x0c => 7,
                0x13 | 0x1f | 0x20 | 0x23 => 2,
                0x26 | 0x28 | 0x2b | 0x30 | 0x32 => 1,
                0x2f | 0x62 => 4,
                0x33 => 0,
                0x60 => 11,
                0x61 => 12,
                _ => {
                    self.fail(
                        frame,
                        kind,
                        format!("unknown action size, block {:02x?}", d),
                    );
                    return;
                }
            };
            if body.len() < size {
                self.fail(frame, kind, format!("truncated action {:02x?}", d));
                return;
            }
            self.apply(w, tags, frame, player, kind, &body[..size]);
            d = &body[size..];
        }
    }

    fn sel_of(&self, tags: &TagMap, tag: u16) -> Sel {
        if let Some(id) = self.bound.get(&tag) {
            return Sel::Unit(*id);
        }
        match tags.id_of(tag) {
            Ok(id) => Sel::Unit(id),
            Err(_) => Sel::Tag(tag),
        }
    }

    fn is_bound(&self, id: u32) -> bool {
        self.bound.values().any(|v| *v == id)
    }

    fn fits(w: &World, u: &galaxycraft::sim::SimUnit, class: Class) -> bool {
        let t = w.data.unit(u.type_id);
        match class {
            Class::Worker => t.is_worker(),
            Class::Military => !t.is_worker() && !t.is_building(),
            Class::Mobile => !t.is_building(),
            Class::AnyBuilding => t.is_building(),
            Class::Producer(ty) => {
                galaxycraft::tech::build_info(ty).is_some_and(|i| i.producer == u.type_id)
            }
        }
    }

    /// Bind the pending tags of `p`'s selection to unbound engine units of
    /// `class`.  Older units have higher tags, so the pending tags are taken
    /// in descending order against the candidates in creation order.
    fn bind_selection(&mut self, w: &World, p: u8, class: Class) {
        let Some(sel) = self.selection.get(&p).cloned() else {
            return;
        };
        let mut pending: Vec<u16> = sel
            .iter()
            .filter_map(|s| if let Sel::Tag(t) = s { Some(*t) } else { None })
            .collect();
        if pending.is_empty() {
            return;
        }
        pending.sort_unstable_by(|a, b| b.cmp(a));
        let candidates: Vec<u32> = w
            .units
            .iter()
            .filter(|u| {
                u.alive
                    && u.owner == p
                    && !u.is_resource()
                    && u.type_id != galaxycraft::ids::unit::VESPENE_GEYSER
                    && !self.is_bound(u.id)
                    && Self::fits(w, u, class)
            })
            .map(|u| u.id)
            .collect();
        // Units already selected by id are not candidates again.
        let taken: Vec<u32> = sel
            .iter()
            .filter_map(|s| if let Sel::Unit(i) = s { Some(*i) } else { None })
            .collect();
        let mut it = candidates.into_iter().filter(|c| !taken.contains(c));
        for tag in pending {
            if let Some(id) = it.next() {
                self.bound.insert(tag, id);
                self.binds += 1;
            }
        }
        let fresh: Vec<Sel> = sel
            .iter()
            .map(|s| match s {
                Sel::Tag(t) => self.bound.get(t).map_or(*s, |id| Sel::Unit(*id)),
                other => *other,
            })
            .collect();
        self.selection.insert(p, fresh);
    }

    /// Alive units of the selection (after binding).
    fn selected(&self, w: &World, p: u8) -> Vec<u32> {
        self.selection
            .get(&p)
            .map(|s| {
                s.iter()
                    .filter_map(|x| match x {
                        Sel::Unit(i) => w.unit(*i).filter(|u| u.alive).map(|u| u.id),
                        Sel::Tag(_) => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// A target tag: a bound/static unit, else the nearest unbound unit of a
    /// different owner (or a resource) around the click.
    fn target_of(
        &mut self,
        w: &World,
        tags: &TagMap,
        p: u8,
        tag: u16,
        pos: (i32, i32),
    ) -> Option<u32> {
        if let Sel::Unit(id) = self.sel_of(tags, tag) {
            return Some(id);
        }
        let near = w
            .units
            .iter()
            .filter(|u| u.alive && u.owner != p && !self.is_bound(u.id))
            .map(|u| {
                (
                    ((u.x / 256 - pos.0).pow(2) + (u.y / 256 - pos.1).pow(2)) as i64,
                    u.id,
                )
            })
            .filter(|(d, _)| *d <= 96 * 96)
            .min()?;
        self.bound.insert(tag, near.1);
        self.binds += 1;
        Some(near.1)
    }

    fn apply(&mut self, w: &mut World, tags: &TagMap, frame: u32, p: u8, kind: u8, b: &[u8]) {
        match kind {
            0x63..=0x65 => {
                let ids: Vec<Sel> = (0..b[0] as usize)
                    .map(|i| self.sel_of(tags, u16_at(b, 1 + 4 * i)))
                    .collect();
                let sel = self.selection.entry(p).or_default();
                match kind {
                    0x63 => *sel = ids,
                    0x64 => sel.extend(ids),
                    _ => sel.retain(|u| !ids.contains(u)),
                }
                self.ok(kind);
            }
            0x13 => {
                let sel = self.selection.get(&p).cloned().unwrap_or_default();
                if b[0] == 0 {
                    self.groups.insert((p, b[1]), sel);
                } else if let Some(g) = self.groups.get(&(p, b[1])).cloned() {
                    self.selection.insert(p, g);
                }
                self.ok(kind);
            }
            0x1f => {
                let ty = u16_at(b, 0);
                self.bind_selection(w, p, Class::Producer(ty));
                let Some(&first) = self.selected(w, p).first() else {
                    let blds: Vec<String> = w
                        .units
                        .iter()
                        .filter(|u| {
                            u.alive
                                && u.owner == p
                                && !u.is_resource()
                                && u.type_id != galaxycraft::ids::unit::VESPENE_GEYSER
                                && w.data.unit(u.type_id).is_building()
                        })
                        .map(|u| {
                            format!(
                                "{}#{}{}",
                                w.data.unit_name(u.type_id),
                                u.id,
                                if u.completed {
                                    String::new()
                                } else {
                                    format!("(building, {} left)", u.build_remaining)
                                }
                            )
                        })
                        .collect();
                    return self.fail(
                        frame,
                        kind,
                        format!("train: no producer in the selection; buildings {blds:?}"),
                    );
                };
                match w.train(first, ty) {
                    Ok(()) => self.ok(kind),
                    Err(e) => self.fail(
                        frame,
                        kind,
                        format!(
                            "train {} in {} failed: {e:?} (minerals {})",
                            w.data.unit_name(ty),
                            w.unit(first)
                                .map(|u| w.data.unit_name(u.type_id))
                                .unwrap_or("?"),
                            w.players[p as usize].minerals
                        ),
                    ),
                }
            }
            0x0c => {
                let ty = u16_at(b, 5);
                let (pw, ph) = {
                    let t = w.data.unit(ty);
                    (t.placement.x as i32, t.placement.y as i32)
                };
                let want = (
                    u16_at(b, 1) as i32 * 32 + pw / 2,
                    u16_at(b, 3) as i32 * 32 + ph / 2,
                );
                self.bind_selection(w, p, Class::Worker);
                let Some(&builder) = self.selected(w, p).first() else {
                    return self.fail(frame, kind, "build: no worker in the selection".into());
                };
                let cost = w.data.unit(ty).mineral_cost as i32;
                let have = w.players[p as usize].minerals;
                if have < cost {
                    // The recorded game had the money (the building exists
                    // later); keep going and count the gap.
                    self.deficit += (cost - have) as i64;
                    w.players[p as usize].minerals = cost;
                }
                match w.placement_spot(ty, want) {
                    None => self.fail(
                        frame,
                        kind,
                        format!("{} cannot be placed at {:?}", w.data.unit_name(ty), want),
                    ),
                    Some(spot) => {
                        if spot != want {
                            self.fail(
                                frame,
                                kind,
                                format!(
                                    "{} placed at {spot:?} instead of {want:?}",
                                    w.data.unit_name(ty)
                                ),
                            );
                        }
                        match w.issue(
                            builder,
                            Order::Build {
                                unit_type: ty,
                                pos: spot,
                            },
                        ) {
                            Ok(()) => self.ok(kind),
                            Err(e) => self.fail(
                                frame,
                                kind,
                                format!("build {} failed: {e:?}", w.data.unit_name(ty)),
                            ),
                        }
                    }
                }
            }
            0x60 | 0x61 => {
                let pos = (u16_at(b, 0) as i32, u16_at(b, 2) as i32);
                let tag = u16_at(b, 4);
                let target = (tag != 0)
                    .then(|| self.target_of(w, tags, p, tag, pos))
                    .flatten();
                let order_id = (kind == 0x61).then(|| b[10]);
                let worker_job = target.and_then(|t| w.unit(t)).is_some_and(|t| {
                    t.is_resource() || t.type_id == galaxycraft::ids::unit::REFINERY
                });
                self.bind_selection(
                    w,
                    p,
                    if worker_job {
                        Class::Worker
                    } else {
                        Class::Mobile
                    },
                );
                let sel = self.selected(w, p);
                if sel.is_empty() {
                    return self.fail(
                        frame,
                        kind,
                        format!(
                            "command: no bound unit in the selection {:?}",
                            self.selection.get(&p)
                        ),
                    );
                }
                for id in sel {
                    match classify(w, id, p, pos, target, order_id) {
                        Some(o) => match w.issue(id, o) {
                            Ok(()) => self.ok(kind),
                            Err(e) => self.fail(frame, kind, format!("issue failed: {e:?}")),
                        },
                        None => *self.unsupported.entry(kind).or_default() += 1,
                    }
                }
            }
            0x20 | 0x26 | 0x2b | 0x2f | 0x30 | 0x32 => {
                let class = match kind {
                    0x26 | 0x2b => Class::Military,
                    _ => Class::AnyBuilding,
                };
                self.bind_selection(w, p, class);
                let Some(&first) = self.selected(w, p).first() else {
                    return self.fail(
                        frame,
                        kind,
                        format!(
                            "command: no bound unit in the selection {:?}",
                            self.selection.get(&p)
                        ),
                    );
                };
                let r = match kind {
                    0x20 => w.dequeue_production(first).map(|_| ()),
                    0x26 => w.issue(first, Order::Siege),
                    0x2b => w.issue(first, Order::HoldPosition),
                    0x2f => w.lift_building(first),
                    0x30 => w.research(first, b[0]),
                    _ => w.upgrade(first, b[0]),
                };
                match r {
                    Ok(()) => self.ok(kind),
                    Err(e) => self.fail(
                        frame,
                        kind,
                        format!(
                            "failed: {e:?} (minerals {})",
                            w.players[p as usize].minerals
                        ),
                    ),
                }
            }
            _ => *self.unsupported.entry(kind).or_default() += 1,
        }
    }

    fn report(&self) {
        println!(
            "tags bound by context: {}; mineral deficit covered for builds: {}",
            self.binds, self.deficit
        );
        println!("applied   {:02x?}", self.applied);
        println!(
            "unsupported (decoded, not executed) {:02x?}",
            self.unsupported
        );
        println!("failures: {}", self.failures.len());
        // Group by message shape (digits stripped) and show the first of each.
        let mut groups: BTreeMap<String, (usize, u32, String)> = BTreeMap::new();
        for (frame, kind, msg) in &self.failures {
            let shape: String = msg.chars().filter(|c| !c.is_ascii_digit()).collect();
            let key = format!("{kind:#04x} {}", shape.split('(').next().unwrap_or(""));
            let e = groups.entry(key).or_insert((0, *frame, msg.clone()));
            e.0 += 1;
        }
        let mut rows: Vec<_> = groups.iter().collect();
        rows.sort_by_key(|(_, (_, frame, _))| *frame);
        for (key, (n, frame, msg)) in rows {
            println!("  x{n:<4} first at frame {frame:>6}  [{key}]  {msg}");
        }
        let _ = SimError::BadUnit;
    }
}

/// The order a right click (or targeted order) means for one selected unit.
fn classify(
    w: &World,
    id: u32,
    player: u8,
    pos: (i32, i32),
    target: Option<u32>,
    order_id: Option<u8>,
) -> Option<Order> {
    let unit = w.unit(id)?;
    if let Some(o) = order_id {
        return match o {
            0x06 => Some(Order::Move { pos }),
            0x0e => Some(Order::AttackMove { pos }),
            _ => None,
        };
    }
    if let Some(t) = target.and_then(|t| w.unit(t)) {
        let worker = w.data.unit(unit.type_id).is_worker();
        if worker && (t.is_resource() || t.type_id == galaxycraft::ids::unit::REFINERY) {
            return Some(Order::Gather { resource: t.id });
        }
        if t.owner != player && t.owner != 0xff && t.owner < 8 && !t.is_resource() {
            return Some(Order::AttackUnit { target: t.id });
        }
    }
    Some(Order::Move { pos })
}

/// Patch the owners of the map's start-location records so player 0 starts
/// where the replay's human actually played.  Returns the patched CHK.
fn assign_start_locations(rep: &RetailReplay) -> Vec<u8> {
    let mut chk = rep.chk.clone();
    // First positional command of player 0: build (tile) or right click (px).
    let first = rep.actions.iter().find_map(|b| {
        let d = &b.data;
        (d.len() >= 9 && d[0] == 0).then(|| match d[1] {
            0x0c => Some((u16_at(d, 3) as i32 * 32, u16_at(d, 5) as i32 * 32)),
            0x60 | 0x61 => Some((u16_at(d, 2) as i32, u16_at(d, 4) as i32)),
            _ => None,
        })?
    });
    let Some((fx, fy)) = first else { return chk };
    let mut o = 0usize;
    while o + 8 <= chk.len() {
        let size = u32::from_le_bytes([chk[o + 4], chk[o + 5], chk[o + 6], chk[o + 7]]) as usize;
        let end = (o + 8 + size).min(chk.len());
        if &chk[o..o + 4] == b"UNIT" {
            let mut locs: Vec<(usize, i32)> = Vec::new();
            let mut off = o + 8;
            while off + 36 <= end {
                if u16::from_le_bytes([chk[off + 8], chk[off + 9]]) == 214 {
                    let x = u16::from_le_bytes([chk[off + 4], chk[off + 5]]) as i32;
                    let y = u16::from_le_bytes([chk[off + 6], chk[off + 7]]) as i32;
                    locs.push((off, (x - fx).pow(2) + (y - fy).pow(2)));
                }
                off += 36;
            }
            if let Some(nearest) = locs.iter().min_by_key(|l| l.1).map(|l| l.0) {
                let mut next = 1u8;
                for (off, _) in &locs {
                    chk[off + 16] = if *off == nearest {
                        0
                    } else {
                        let v = next;
                        next += 1;
                        v
                    };
                }
            }
        }
        o = end;
    }
    chk
}
