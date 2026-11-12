//! Controller UI in the layout of the Baldur's Gate 3 pad interface.
//!
//! * LB / RB open the **action radials** (the selection's command card and
//!   its build cards), then step to the previous / next radial.
//! * RT toggles the **shortcut radial** (menu, objectives, idle unit, last
//!   alert, base, minimap, saved cameras).
//! * An LT tap toggles the **group radial** (the "party"); holding LT stays
//!   the add / queue modifier.
//! * A radial has the twelve clock slots; the left stick points at one, A
//!   uses it, B closes.  The button prompts sit bottom-right.
//!
//! Every proportion below is a fraction of the 16:9 content height, measured
//! on a 1920x1080 capture of that interface, so the radials keep the same
//! on-screen size at any resolution.

use super::*;
use galaxycraft::menu::LabelAlign;

/// Centre of the focused radial (x is the content centre).
const RADIAL_CENTRE_Y: f32 = 0.505;
/// Outer / inner radius of the focused radial (432 / 288 px wide at 1080p).
const RADIAL_OUTER: f32 = 0.200;
const RADIAL_INNER: f32 = 0.133;
/// The neighbouring radials are drawn at 64 % of that size, dimmed.
const SIDE_OUTER: f32 = 0.128;
const SIDE_INNER: f32 = 0.089;
/// Centre distance focused -> first neighbour -> second neighbour.
const SIDE_SPACING: f32 = 0.430;
const SIDE_SPACING_FAR: f32 = 0.395;
/// LB / RB tags between the focused radial and its neighbours.
const BUMPER_TAG_DX: f32 = 0.264;
const BUMPER_TAG_DY: f32 = -0.160;
/// Description panel under the focused radial.
const TOOLTIP_TOP: f32 = 0.218;
const TOOLTIP_W: f32 = 0.650;
/// Button prompts: right margin, row pitch, lowest row, glyph radius.
const PROMPT_RIGHT: f32 = 0.051;
const PROMPT_PITCH: f32 = 0.048;
const PROMPT_BOTTOM: f32 = 0.955;
const PROMPT_GLYPH_R: f32 = 0.013;
/// Text em sizes.
const EM_LABEL: f32 = 0.0205;
const EM_BODY: f32 = 0.0175;
const EM_TAG: f32 = 0.0150;

const SLOTS: usize = gamepad::RADIAL_SLOTS;

/// Which radial set is open.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum RadialKind {
    #[default]
    Actions,
    Shortcuts,
    Groups,
}

/// Entries of the RT shortcut radial.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PadShortcut {
    Menu,
    Objectives,
    IdleUnit,
    LastAlert,
    Base,
    Minimap,
    Camera(usize),
    SaveCamera(usize),
}

impl PadShortcut {
    /// Clockwise from twelve o'clock.
    const ALL: [PadShortcut; 12] = [
        PadShortcut::Menu,
        PadShortcut::Objectives,
        PadShortcut::IdleUnit,
        PadShortcut::LastAlert,
        PadShortcut::Base,
        PadShortcut::Minimap,
        PadShortcut::Camera(0),
        PadShortcut::Camera(1),
        PadShortcut::Camera(2),
        PadShortcut::SaveCamera(0),
        PadShortcut::SaveCamera(1),
        PadShortcut::SaveCamera(2),
    ];

    fn label(self) -> String {
        match self {
            PadShortcut::Menu => "Game Menu".into(),
            PadShortcut::Objectives => "Objectives".into(),
            PadShortcut::IdleUnit => "Idle Unit".into(),
            PadShortcut::LastAlert => "Last Alert".into(),
            PadShortcut::Base => "Main Base".into(),
            PadShortcut::Minimap => "Minimap".into(),
            PadShortcut::Camera(i) => format!("Camera {}", i + 1),
            PadShortcut::SaveCamera(i) => format!("Save Camera {}", i + 1),
        }
    }

    fn tooltip(self) -> &'static str {
        match self {
            PadShortcut::Menu => "Open the game menu: save, load, options, end mission.",
            PadShortcut::Objectives => "Show the mission objectives and the score.",
            PadShortcut::IdleUnit => "Select the next idle worker, or the next idle army unit.",
            PadShortcut::LastAlert => "Move the screen to the last alert or transmission.",
            PadShortcut::Base => "Centre the screen on your main base.",
            PadShortcut::Minimap => "Move the pointer on the minimap. A jumps there, B leaves.",
            PadShortcut::Camera(_) => "Jump to this saved camera position.",
            PadShortcut::SaveCamera(_) => "Store the current screen position in this camera.",
        }
    }

    /// Short text drawn in the slot (there is no retail icon for these).
    fn glyph(self) -> String {
        match self {
            PadShortcut::Menu => "MENU".into(),
            PadShortcut::Objectives => "OBJ".into(),
            PadShortcut::IdleUnit => "IDLE".into(),
            PadShortcut::LastAlert => "ALERT".into(),
            PadShortcut::Base => "BASE".into(),
            PadShortcut::Minimap => "MAP".into(),
            PadShortcut::Camera(i) => format!("C{}", i + 1),
            PadShortcut::SaveCamera(i) => format!("S{}", i + 1),
        }
    }
}

/// What a radial slot does when A is pressed on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PadAction {
    /// A retail command-card slot (`WheelEntry::set` / `slot`).
    Card,
    Shortcut(PadShortcut),
    /// Control group index into `App::groups`.
    Group(usize),
}

/// Button glyph of a prompt row.
#[derive(Clone, Copy)]
enum Glyph {
    A,
    B,
    X,
    Y,
    Tag(&'static str),
}

/// Geometry of one radial in window pixels.
#[derive(Clone, Copy)]
struct Ring {
    cx: i32,
    cy: i32,
    outer: f32,
    inner: f32,
    dim: bool,
}

/// Angle of the offset `(dx, dy)` in slot units: `0.0..12.0`, where the
/// integer part is the clock slot and slot 0 is centred on twelve o'clock.
fn slot_turn(dx: f32, dy: f32) -> f32 {
    let step = std::f32::consts::TAU / SLOTS as f32;
    (dy.atan2(dx) + std::f32::consts::FRAC_PI_2 + step / 2.0).rem_euclid(std::f32::consts::TAU)
        / step
}

/// Clock slot under the offset `(dx, dy)` from the ring centre.
#[cfg(test)]
fn slot_at(dx: f32, dy: f32) -> usize {
    (slot_turn(dx, dy) as usize).min(SLOTS - 1)
}

/// Centre of clock slot `i` at radius `r`.
fn slot_centre(ring: &Ring, i: usize, r: f32) -> (i32, i32) {
    let a = i as f32 * std::f32::consts::TAU / SLOTS as f32 - std::f32::consts::FRAC_PI_2;
    let (sin, cos) = a.sin_cos();
    (
        ring.cx + (cos * r).round() as i32,
        ring.cy + (sin * r).round() as i32,
    )
}

fn scale_rgb(c: (f32, f32, f32), k: f32) -> Rgb {
    Rgb::new(
        (c.0 * k).clamp(0.0, 255.0) as u8,
        (c.1 * k).clamp(0.0, 255.0) as u8,
        (c.2 * k).clamp(0.0, 255.0) as u8,
    )
}

/// Rasterise one radial: a see-through vignetted centre, twelve wedges
/// (`filled[i]` = the slot holds something), the focused wedge in rust
/// orange, dark separators and gold rims.
fn draw_ring(fb: &mut Framebuffer, ring: &Ring, filled: &[bool; SLOTS], focus: Option<usize>) {
    let step = std::f32::consts::TAU / SLOTS as f32;
    let rim = (ring.outer * 0.012).max(1.2);
    let sep = (ring.outer * 0.006).max(0.7);
    let shade = if ring.dim { 0.62 } else { 1.0 };
    let band_alpha = if ring.dim { 178.0 } else { 232.0 };
    let r = ring.outer.ceil() as i32 + 1;
    let x0 = (ring.cx - r).max(0);
    let x1 = (ring.cx + r).min(fb.width - 1);
    let y0 = (ring.cy - r).max(0);
    let y1 = (ring.cy + r).min(fb.height - 1);
    for y in y0..=y1 {
        let dy = (y - ring.cy) as f32;
        for x in x0..=x1 {
            let dx = (x - ring.cx) as f32;
            let d = (dx * dx + dy * dy).sqrt();
            if d > ring.outer + 0.5 {
                continue;
            }
            let (color, alpha) = if d < ring.inner - rim {
                // The world stays visible through the middle.
                let t = d / ring.inner;
                let a = if ring.dim { 120.0 } else { 34.0 } + 120.0 * t * t * t;
                (Rgb::new(4, 6, 4), a)
            } else if d < ring.inner || d > ring.outer - rim {
                (scale_rgb((156.0, 132.0, 86.0), shade), 245.0)
            } else {
                let a = slot_turn(dx, dy);
                let i = (a as usize).min(SLOTS - 1);
                let frac = a - a.floor();
                let edge = frac.min(1.0 - frac) * step * d;
                if edge < sep {
                    (Rgb::new(10, 14, 8), 245.0)
                } else {
                    // Lit towards the rim, like the reference bands.
                    let t = (d - ring.inner) / (ring.outer - ring.inner);
                    let lit = (0.78 + 0.40 * t) * shade;
                    let base = if focus == Some(i) {
                        (176.0, 74.0, 28.0)
                    } else if filled[i] {
                        (36.0, 54.0, 24.0)
                    } else {
                        (22.0, 30.0, 17.0)
                    };
                    (scale_rgb(base, lit), band_alpha)
                }
            };
            let coverage = (ring.outer + 0.5 - d).clamp(0.0, 1.0);
            let a = (alpha * coverage).clamp(0.0, 255.0) as u8;
            let under = fb.get(x, y);
            fb.put(x, y, under.blend(color, a));
        }
    }
}

/// Greedy word wrap to at most `max_chars` per line.
fn wrap_text(text: &str, max_chars: usize, max_lines: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > max_chars {
            lines.push(std::mem::take(&mut line));
            if lines.len() == max_lines {
                return lines;
            }
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

fn plain_entry(action: PadAction, label: String, tooltip: String, enabled: bool) -> WheelEntry {
    WheelEntry {
        set: None,
        slot: 0,
        act: galaxycraft::buttons::Act::of(0),
        label,
        tooltip,
        icon: u16::MAX,
        enabled,
        action,
    }
}

impl App {
    // ---------------------------------------------------------------- input

    /// Controller-first gameplay input.  Returns true when the action opened
    /// a modal or consumed the frame, so the rest of the frame must stop.
    pub(super) fn handle_pad_input(
        &mut self,
        input: &GameInput,
        mx: i32,
        my: i32,
        in_view: bool,
    ) -> bool {
        let pad = &input.pad;
        let now_ms = self.pad_started.elapsed().as_millis() as u64;
        let lt_tap = self.pad_lt_tap.update(now_ms, pad.shift, pad.chord);
        if pad.menu {
            self.close_pad_radial();
            self.pad_minimap_mode = false;
            self.open_game_dialog(GameDialog::Menu);
            self.play("ui_click");
            return true;
        }
        if self.pad_wheel.is_open() {
            self.handle_pad_radial(pad, lt_tap);
            // A radial is modal: no other game input runs while it is open,
            // and the pointer stays hidden until it closes.
            return true;
        }
        if pad.page_prev || pad.page_next {
            self.pad_minimap_mode = false;
            self.open_pad_radial(RadialKind::Actions);
            return true;
        }
        if pad.shortcut_radial {
            self.pad_minimap_mode = false;
            self.open_pad_radial(RadialKind::Shortcuts);
            return true;
        }
        if lt_tap {
            self.pad_minimap_mode = false;
            self.open_pad_radial(RadialKind::Groups);
            return true;
        }

        // View: the pointer moves on the minimap ("map"); A jumps the
        // camera there, B or View leaves.
        if pad.map {
            self.pad_minimap_mode = !self.pad_minimap_mode;
            self.play("ui_click");
            return true;
        }
        if self.pad_minimap_mode {
            if pad.primary {
                self.jump_camera_to_minimap(mx, my);
                self.play("ui_click");
            }
            if pad.cancel {
                self.pad_minimap_mode = false;
                self.play("ui_click");
            }
            return true;
        }

        // D-pad: left / right step through the control groups, up jumps to
        // the last alert, down examines (centres on) the selection.
        if pad.nav_pressed.0 || pad.nav_pressed.1 {
            self.pad_step_group(if pad.nav_pressed.0 { -1 } else { 1 });
            return true;
        }
        if pad.nav_pressed.2 {
            self.jump_to_last_alert();
            return true;
        }
        if pad.nav_pressed.3 {
            if let Some((x, y)) = self.first_selected().map(|u| (u.x / 256, u.y / 256)) {
                self.jump_camera_to(x, y);
                self.play("ui_move");
            } else {
                self.status("No unit selected");
            }
            return true;
        }

        // X: context order (move / attack / gather).  Y: attack-move.
        if pad.smart_order || pad.attack_move {
            if self.target_act.is_some() || self.build_mode.is_some() {
                self.status("Confirm with A or cancel with B");
            } else if in_view {
                self.command_at(mx, my, pad.attack_move, false, input.shift);
            }
            return true;
        }

        // A: press starts a click/drag; release selects a unit or a box.
        if pad.cancel {
            self.pad_cancel();
            return true;
        }
        if pad.primary {
            if in_view {
                if self.target_act.is_some() {
                    self.execute_target(mx, my);
                } else if self.build_mode.is_some() {
                    self.place_building(mx, my);
                } else {
                    self.pad_drag = Some((mx, my, pad.shift));
                    self.drag_start = Some((mx, my));
                    self.drag_now = (mx, my);
                }
            } else {
                self.pad_primary(input, mx, my, false);
            }
            return true;
        }
        if self.pad_drag.is_some() && pad.lmb {
            // A is held: keep the drag box alive and visible until release.
            self.drag_now = (mx, my);
            return true;
        }
        if let Some((start_x, start_y, additive)) = self.pad_drag
            && !pad.lmb
        {
            self.pad_drag = None;
            self.drag_start = None;
            let start = (start_x, start_y);
            let dist = (mx - start.0).abs() + (my - start.1).abs();
            if dist < 6 {
                self.pad_click_select(mx, my, in_view, additive);
            } else {
                self.select_box_region(start, (mx, my), additive);
            }
            return true;
        }
        false
    }

    /// Input while a radial is open.
    fn handle_pad_radial(&mut self, pad: &gamepad::PadFrame, lt_tap: bool) {
        let kind = self.pad_wheel_kind;
        // The triggers toggle their own radial and switch from another one.
        if pad.shortcut_radial {
            if kind == RadialKind::Shortcuts {
                self.close_pad_radial();
            } else {
                self.open_pad_radial(RadialKind::Shortcuts);
            }
            return;
        }
        if lt_tap {
            if kind == RadialKind::Groups {
                self.close_pad_radial();
            } else {
                self.open_pad_radial(RadialKind::Groups);
            }
            return;
        }
        if pad.page_prev || pad.page_next {
            if kind != RadialKind::Actions {
                self.open_pad_radial(RadialKind::Actions);
            } else if self
                .pad_wheel
                .page_delta(if pad.page_prev { -1 } else { 1 })
            {
                self.play("ui_move");
            }
            return;
        }
        let mut moved = false;
        // Left stick first (as on the reference pad), right stick as well.
        for (dx, dy) in [pad.stick, pad.aim] {
            if dx != 0.0 || dy != 0.0 {
                moved |= self.pad_wheel.aim(dx, dy);
            }
        }
        let delta = if pad.nav_pressed.0 || pad.nav_pressed.2 {
            -1
        } else if pad.nav_pressed.1 || pad.nav_pressed.3 {
            1
        } else {
            0
        };
        if delta != 0 {
            // The D-pad skips the empty slots.
            let count = self
                .pad_wheel_pages
                .get(self.pad_wheel.page())
                .map_or(0, |p| p.entries.len().min(SLOTS));
            if count > 0 {
                let at = self.pad_wheel.focus().min(count - 1) as isize;
                let from_empty = self.pad_wheel.focus() >= count;
                let next = if from_empty {
                    if delta > 0 { 0 } else { count as isize - 1 }
                } else {
                    (at + delta).rem_euclid(count as isize)
                };
                moved |= next as usize != self.pad_wheel.focus();
                self.pad_wheel.set_focus(next as usize);
            }
        }
        if moved {
            self.play("ui_move");
        }
        if pad.primary {
            self.activate_pad_radial();
        } else if pad.cancel {
            self.close_pad_radial();
        } else if pad.smart_order && kind == RadialKind::Groups {
            self.assign_focused_group();
        }
    }

    /// D-pad left / right: recall the previous / next non-empty group.
    fn pad_step_group(&mut self, delta: i32) {
        let live = |app: &App, g: usize| {
            app.groups[g]
                .iter()
                .any(|id| app.world.unit(*id).is_some_and(|u| u.alive))
        };
        // Display order 1..9, 0.
        let order: Vec<usize> = (1..10).chain([0]).filter(|g| live(self, *g)).collect();
        if order.is_empty() {
            self.status("No control groups: LT opens the group radial, X assigns");
            return;
        }
        let at = order.iter().position(|g| *g == self.pad_group_at);
        let next = match at {
            Some(i) => (i as i32 + delta).rem_euclid(order.len() as i32) as usize,
            None if delta > 0 => 0,
            None => order.len() - 1,
        };
        self.pad_group_at = order[next];
        self.recall_group(order[next]);
        self.centre_on_group(order[next]);
    }

    // -------------------------------------------------------------- radials

    fn action_radial_pages(&mut self) -> Vec<WheelPage> {
        use galaxycraft::buttons::{Act, card_of_set};
        if self.target_act.is_some() || self.build_mode.is_some() {
            self.status("Finish or cancel the current order first");
            return Vec::new();
        }
        let units: Vec<SimUnit> = self.selected_units().into_iter().cloned().collect();
        if units.is_empty() {
            self.status("No unit selected");
            return Vec::new();
        }
        if units.iter().any(|u| u.owner as usize != self.local_player) {
            self.status("No commands for this selection");
            return Vec::new();
        }
        let mut pages = Vec::new();
        let first = wheel_page_for_selection(&self.world, &units, self.card_set);
        if !first.entries.is_empty() {
            pages.push(first);
        }
        // A lone unit also gets one radial per build card its own card
        // links to, so every structure is one bumper press away.
        if units.len() == 1 && self.card_set.is_none() {
            let base = card_of_set(&self.world, &units[0], None);
            let mut sets: Vec<u16> = Vec::new();
            for (b, _) in base.iter().flatten() {
                if Act::of(b.act) == Act::Card && b.act_var != 228 && !sets.contains(&b.act_var) {
                    sets.push(b.act_var);
                }
            }
            for (n, set) in sets.into_iter().enumerate() {
                let mut page = wheel_page_for_selection(&self.world, &units, Some(set));
                page.entries.retain(|e| e.act != Act::Card);
                page.title = match n {
                    0 => "Structures",
                    1 => "Advanced Structures",
                    _ => "More Commands",
                };
                if !page.entries.is_empty() {
                    pages.push(page);
                }
            }
        }
        if pages.is_empty() {
            self.status("No shared commands");
        }
        pages
    }

    fn shortcut_radial_page(&self) -> WheelPage {
        let entries = PadShortcut::ALL
            .iter()
            .map(|s| {
                let enabled = match s {
                    PadShortcut::Camera(i) => self.camera_slot_target(*i).is_some(),
                    _ => true,
                };
                plain_entry(
                    PadAction::Shortcut(*s),
                    s.label(),
                    s.tooltip().to_string(),
                    enabled,
                )
            })
            .collect();
        WheelPage {
            title: "Shortcuts",
            entries,
        }
    }

    fn group_radial_page(&self) -> WheelPage {
        let entries = (1..10)
            .chain([0])
            .map(|g| {
                let alive: Vec<&SimUnit> = self.groups[g]
                    .iter()
                    .filter_map(|id| self.world.unit(*id))
                    .filter(|u| u.alive)
                    .collect();
                let tooltip = match alive.first() {
                    None => "Empty. X assigns the current selection to this group.".to_string(),
                    Some(u) => format!(
                        "{} x {}{}. A selects the group, X replaces it with the selection.",
                        alive.len(),
                        self.data.unit_name(u.type_id),
                        if alive.iter().any(|o| o.type_id != u.type_id) {
                            " and others"
                        } else {
                            ""
                        }
                    ),
                };
                plain_entry(
                    PadAction::Group(g),
                    format!("Group {g}"),
                    tooltip,
                    !alive.is_empty(),
                )
            })
            .collect();
        WheelPage {
            title: "Control Groups",
            entries,
        }
    }

    pub(super) fn open_pad_radial(&mut self, kind: RadialKind) {
        let pages = match kind {
            RadialKind::Actions => self.action_radial_pages(),
            RadialKind::Shortcuts => vec![self.shortcut_radial_page()],
            RadialKind::Groups => vec![self.group_radial_page()],
        };
        if pages.is_empty() {
            self.pad_wheel.close();
            self.pad_wheel_pages.clear();
            return;
        }
        // Every radial has the twelve clock slots, filled or not.
        self.pad_wheel.open(pages.iter().map(|_| SLOTS));
        self.pad_wheel_pages = pages;
        self.pad_wheel_kind = kind;
        self.play("ui_click");
    }

    pub(super) fn close_pad_radial(&mut self) {
        if self.pad_wheel.is_open() {
            self.play("ui_click");
        }
        self.pad_wheel.close();
        self.pad_wheel_pages.clear();
    }

    fn pad_focused_entry(&self) -> Option<&WheelEntry> {
        self.pad_wheel_pages
            .get(self.pad_wheel.page())
            .and_then(|p| p.entries.get(self.pad_wheel.focus()))
    }

    /// A while a radial is open: use the focused slot.  Empty and disabled
    /// slots keep the radial open so the player can pick another one.
    fn activate_pad_radial(&mut self) {
        let Some(entry) = self.pad_focused_entry().cloned() else {
            return;
        };
        match entry.action {
            PadAction::Card => {
                if !entry.enabled {
                    self.status(format!("{} is not available yet", entry.label));
                    return;
                }
                if self.pad_wheel.activate().is_none() {
                    return;
                }
                self.pad_wheel_pages.clear();
                let is_card = entry.act == galaxycraft::buttons::Act::Card;
                // `activate_command` reads the card through `card_set`;
                // point it at the entry's real set, then restore the HUD's
                // page.  A card-switching button keeps the card it selected
                // and reopens the radials on it.
                let old_set = self.card_set;
                self.card_set = entry.set.filter(|s| *s != 228);
                self.activate_command(entry.slot);
                if is_card {
                    self.open_pad_radial(RadialKind::Actions);
                } else {
                    self.card_set = old_set;
                }
                self.pad_tap.reset();
            }
            PadAction::Shortcut(shortcut) => {
                if !entry.enabled {
                    self.status(format!("{} is not saved yet", entry.label));
                    return;
                }
                self.close_pad_radial();
                self.run_pad_shortcut(shortcut);
            }
            PadAction::Group(g) => {
                if !entry.enabled {
                    self.status(format!("Group {g} is empty: X assigns the selection"));
                    return;
                }
                self.close_pad_radial();
                let was_selected = !self.selected.is_empty()
                    && self.selected.iter().all(|id| self.groups[g].contains(id));
                self.recall_group(g);
                self.pad_group_at = g;
                if was_selected {
                    self.centre_on_group(g);
                }
            }
        }
    }

    fn run_pad_shortcut(&mut self, shortcut: PadShortcut) {
        match shortcut {
            PadShortcut::Menu => self.open_game_dialog(GameDialog::Menu),
            PadShortcut::Objectives => self.open_game_dialog(GameDialog::Objectives),
            PadShortcut::IdleUnit => self.cycle_idle_unit(),
            PadShortcut::LastAlert => self.jump_to_last_alert(),
            PadShortcut::Base => self.center_on_base(),
            PadShortcut::Minimap => {
                self.pad_minimap_mode = true;
                self.status("Minimap: A jumps there, B leaves");
            }
            PadShortcut::Camera(slot) => {
                if let Some(pos) = self.camera_slot_target(slot) {
                    self.jump_camera_to(pos.0, pos.1);
                    self.status(format!("Camera {}", slot + 1));
                    self.play("ui_move");
                }
            }
            PadShortcut::SaveCamera(slot) => self.save_camera_slot(slot),
        }
    }

    /// X in the group radial: the selection becomes the focused group.
    fn assign_focused_group(&mut self) {
        let Some(PadAction::Group(g)) = self.pad_focused_entry().map(|e| e.action) else {
            return;
        };
        if self.selected.is_empty() {
            self.status("Select units first");
            return;
        }
        self.groups[g] = self.selected.iter().take(12).copied().collect();
        self.pad_group_at = g;
        self.status(format!("Group {g} assigned"));
        self.play("ui_click");
        // Refresh the radial so the slot shows as filled.
        let focus = self.pad_wheel.focus();
        self.pad_wheel_pages = vec![self.group_radial_page()];
        self.pad_wheel.set_focus(focus);
    }

    // -------------------------------------------------------------- drawing

    /// One label in window pixels; `em` is a fraction of the content height.
    #[allow(clippy::too_many_arguments)]
    fn pad_label(
        &mut self,
        fb: &mut Framebuffer,
        text: &str,
        x: i32,
        top: i32,
        em: f32,
        color: Rgb,
        align: i32,
    ) -> i32 {
        let (scale, ox, oy) = self.ui_metrics(fb);
        let font_ui = em * 480.0;
        let w = self.hd_advance_px(text, font_ui, scale, false);
        let wpx = (w * scale).round() as i32;
        let (ax, mode) = match align {
            0 => (x, LabelAlign::Center),
            1 => (x - wpx, LabelAlign::Left),
            _ => (x, LabelAlign::Left),
        };
        self.draw_hd_text(
            fb,
            text,
            (ax - ox) as f32 / scale,
            (top - oy) as f32 / scale,
            mode,
            w,
            font_ui,
            scale,
            ox,
            oy,
            color,
            false,
        );
        wpx
    }

    /// Icon or short text of one slot, centred on `(x, y)`.
    #[allow(clippy::too_many_arguments)]
    fn draw_radial_slot(
        &mut self,
        fb: &mut Framebuffer,
        entry: &WheelEntry,
        (x, y): (i32, i32),
        size: i32,
        focused: bool,
        dim: bool,
        h: f32,
    ) {
        let color = if !entry.enabled {
            Rgb::new(104, 108, 98)
        } else if dim {
            Rgb::new(150, 138, 100)
        } else if focused {
            Rgb::new(255, 244, 208)
        } else {
            Rgb::new(246, 214, 132)
        };
        if entry.action == PadAction::Card {
            if let Some(img) = self
                .console
                .as_ref()
                .and_then(|c| c.cmdicons4.as_ref())
                .and_then(|g| g.frame(entry.icon as usize))
            {
                draw_tinted(fb, img, (x - size / 2, y - size / 2, size, size), color);
            }
            return;
        }
        let text = match entry.action {
            PadAction::Shortcut(s) => s.glyph(),
            PadAction::Group(g) => g.to_string(),
            PadAction::Card => String::new(),
        };
        let em = if text.chars().count() <= 2 {
            size as f32 / h * 0.74
        } else {
            size as f32 / h * 0.40
        };
        let top = y - (em * h * 0.50) as i32;
        self.pad_label(fb, &text, x, top, em, color, 0);
    }

    /// A small rounded tag such as `LB` / `RT`, centred on `(x, y)`.
    fn draw_pad_tag(&mut self, fb: &mut Framebuffer, text: &str, x: i32, y: i32, h: f32) {
        let (scale, _, _) = self.ui_metrics(fb);
        let w = (self.hd_advance_px(text, EM_TAG * 480.0, scale, false) * scale) as i32;
        let bw = w + (h * 0.012) as i32;
        let bh = (h * 0.024) as i32;
        draw_rounded_box_alpha(
            fb,
            (x - bw / 2, y - bh / 2, bw, bh),
            (h * 0.005).max(2.0),
            1.0,
            Rgb::new(232, 228, 216),
            Rgb::new(232, 228, 216),
            235,
        );
        let top = y - (EM_TAG * h * 0.50) as i32;
        let ink = Rgb::new(18, 18, 16);
        // Dark ink on a light plate: no drop shadow, unlike the labels.
        if let Some(font) = self.ui_font.as_mut() {
            let px = (EM_TAG * 480.0 * scale).max(7.0);
            font.draw_centered(fb, x, top, text, px, 0.0, ink, None);
        } else {
            self.pad_label(fb, text, x, top, EM_TAG, ink, 0);
        }
    }

    /// The open radials: the focused one in the centre, its neighbours
    /// dimmed on both sides, the slot description underneath.
    pub(super) fn draw_pad_radials(&mut self, fb: &mut Framebuffer) {
        if self.game_dialog.is_some() || self.paused || self.game_result.is_some() {
            return;
        }
        if !self.pad_wheel.is_open() {
            return;
        }
        let page_at = self.pad_wheel.page();
        if self.pad_wheel_pages.get(page_at).is_none() {
            self.pad_wheel.close();
            return;
        }
        let (cx, cy, cw, ch) = Self::content_rect(fb);
        let h = ch as f32;
        // The reference has no console under the radial; keep the whole
        // group (radial + description panel) above this one.
        let panel_h = 0.012 * 2.0 + EM_LABEL * 1.5 + EM_BODY * 1.45 * 3.0;
        let fit = (self.playfield_bottom() - cy) as f32 / h - TOOLTIP_TOP - panel_h - 0.006;
        let centre_y = RADIAL_CENTRE_Y.min(fit.max(RADIAL_OUTER + 0.08));
        let centre = (cx + cw / 2, cy + (h * centre_y) as i32);
        let pages = self.pad_wheel_pages.clone();
        let focus = self.pad_wheel.focus();

        // Neighbours first, so the focused radial overlaps them.
        for offset in [-2i32, 2, -1, 1] {
            let at = page_at as i32 + offset;
            if at < 0 || at as usize >= pages.len() {
                continue;
            }
            let reach = SIDE_SPACING + SIDE_SPACING_FAR * (offset.abs() - 1) as f32;
            let ring = Ring {
                cx: centre.0 + (h * reach) as i32 * offset.signum(),
                cy: centre.1,
                outer: h * SIDE_OUTER,
                inner: h * SIDE_INNER,
                dim: true,
            };
            self.draw_radial(fb, &ring, &pages[at as usize], None, h);
        }
        let ring = Ring {
            cx: centre.0,
            cy: centre.1,
            outer: h * RADIAL_OUTER,
            inner: h * RADIAL_INNER,
            dim: false,
        };
        let page = &pages[page_at];
        self.draw_radial(fb, &ring, page, Some(focus), h);

        // Bumper tags with their arrows, between the radials.
        if self.pad_wheel_kind == RadialKind::Actions && pages.len() > 1 {
            let ty = centre.1 + (h * BUMPER_TAG_DY) as i32;
            let dx = (h * BUMPER_TAG_DX) as i32;
            let arrow = Rgb::new(232, 228, 216);
            self.draw_pad_tag(fb, "LB", centre.0 - dx, ty, h);
            self.draw_pad_tag(fb, "RB", centre.0 + dx, ty, h);
            let ay = ty + (h * 0.020) as i32;
            self.pad_label(fb, "<", centre.0 - dx, ay, EM_LABEL, arrow, 0);
            self.pad_label(fb, ">", centre.0 + dx, ay, EM_LABEL, arrow, 0);
        }

        // Heading above the radial (the reference shows the character
        // name there).
        let title = match self.pad_wheel.pages() {
            0 | 1 => page.title.to_string(),
            n => format!("{}  {}/{}", page.title, page_at + 1, n),
        };
        let title_top = centre.1 - (h * (RADIAL_OUTER + 0.048)) as i32;
        self.pad_label(
            fb,
            &title,
            centre.0,
            title_top,
            EM_LABEL,
            Rgb::new(236, 228, 208),
            0,
        );

        // Focused slot: its name inside the ring, the description below.
        let Some(entry) = page.entries.get(focus) else {
            return;
        };
        let name_color = if entry.enabled {
            Rgb::new(244, 240, 228)
        } else {
            Rgb::new(150, 150, 142)
        };
        let name_top = centre.1 + (h * RADIAL_INNER * 0.20) as i32;
        self.pad_label(
            fb,
            &entry.label,
            centre.0,
            name_top,
            EM_LABEL,
            name_color,
            0,
        );

        let lines = wrap_text(&entry.tooltip, 58, 3);
        let line_h = (h * EM_BODY * 1.45) as i32;
        let pad_y = (h * 0.012) as i32;
        let title_h = (h * EM_LABEL * 1.5) as i32;
        let pw = (h * TOOLTIP_W) as i32;
        let ph = pad_y * 2 + title_h + line_h * lines.len() as i32;
        let px = centre.0 - pw / 2;
        let py = centre.1 + (h * TOOLTIP_TOP) as i32;
        draw_rounded_box_alpha(
            fb,
            (px, py, pw, ph),
            (h * 0.004).max(2.0),
            1.0,
            Rgb::new(10, 9, 8),
            Rgb::new(96, 82, 56),
            214,
        );
        self.pad_label(
            fb,
            &entry.label,
            centre.0,
            py + pad_y,
            EM_LABEL,
            name_color,
            0,
        );
        for (i, line) in lines.iter().enumerate() {
            self.pad_label(
                fb,
                line,
                centre.0,
                py + pad_y + title_h + line_h * i as i32,
                EM_BODY,
                Rgb::new(206, 196, 172),
                0,
            );
        }
    }

    /// One radial with its slot icons.
    fn draw_radial(
        &mut self,
        fb: &mut Framebuffer,
        ring: &Ring,
        page: &WheelPage,
        focus: Option<usize>,
        h: f32,
    ) {
        let mut filled = [false; SLOTS];
        for slot in filled.iter_mut().take(page.entries.len()) {
            *slot = true;
        }
        draw_ring(fb, ring, &filled, focus);
        let band = ring.outer - ring.inner;
        let mid = (ring.outer + ring.inner) / 2.0;
        let size = (band * 0.66) as i32;
        for (i, entry) in page.entries.iter().take(SLOTS).enumerate() {
            let at = slot_centre(ring, i, mid);
            self.draw_radial_slot(fb, entry, at, size, focus == Some(i), ring.dim, h);
        }
    }

    /// Button prompts, right-aligned in the bottom-right corner: the label,
    /// then the button glyph.  The rows depend on what the pad does now.
    pub(super) fn draw_pad_prompts(&mut self, fb: &mut Framebuffer) {
        if !self.pad_control || self.game_dialog.is_some() || self.paused {
            return;
        }
        let rows: Vec<(Glyph, &str)> = if self.pad_wheel.is_open() {
            let mut rows = vec![(Glyph::A, "Select")];
            match self.pad_wheel_kind {
                RadialKind::Groups => rows.push((Glyph::X, "Assign Selection")),
                RadialKind::Actions if self.pad_wheel_pages.len() > 1 => {
                    rows.push((Glyph::Tag("LB RB"), "Switch Radial"));
                }
                _ => {}
            }
            rows.push((Glyph::B, "Close"));
            rows
        } else if self.pad_minimap_mode {
            vec![(Glyph::A, "Jump"), (Glyph::B, "Leave Minimap")]
        } else if self.target_act.is_some() || self.build_mode.is_some() {
            vec![(Glyph::A, "Confirm"), (Glyph::B, "Cancel")]
        } else {
            vec![
                (Glyph::Tag("LB RB"), "Actions"),
                (Glyph::Tag("RT"), "Shortcuts"),
                (Glyph::Tag("LT"), "Groups"),
                (Glyph::A, "Select"),
                (Glyph::X, "Order"),
                (Glyph::Y, "Attack"),
                (Glyph::B, "Cancel"),
            ]
        };
        let (cx, cy, cw, ch) = Self::content_rect(fb);
        let h = ch as f32;
        let (scale, _, _) = self.ui_metrics(fb);
        let pitch = (h * PROMPT_PITCH) as i32;
        let r = (h * PROMPT_GLYPH_R) as i32;
        // The reference keeps the list in the bottom-right corner; here the
        // command console sits there, so the list rests on top of it.
        let slots = self.command_slots();
        let console_top = slots
            .first()
            .map_or(cy + ch, |s| s.y - (22.0 * scale).round() as i32);
        let bottom = (cy + (h * PROMPT_BOTTOM) as i32).min(console_top - pitch / 2);
        let right = cx + cw - (h * PROMPT_RIGHT) as i32;
        let n = rows.len() as i32;
        for (i, (glyph, label)) in rows.iter().enumerate() {
            let y = bottom - pitch * (n - 1 - i as i32);
            let glyph_w = match glyph {
                Glyph::Tag(text) => {
                    let w = (self.hd_advance_px(text, EM_TAG * 480.0, scale, false) * scale) as i32
                        + (h * 0.012) as i32;
                    self.draw_pad_tag(fb, text, right - w / 2, y, h);
                    w
                }
                face => {
                    let (letter, color) = match face {
                        Glyph::A => ("A", Rgb::new(120, 214, 110)),
                        Glyph::B => ("B", Rgb::new(236, 96, 88)),
                        Glyph::X => ("X", Rgb::new(104, 168, 244)),
                        _ => ("Y", Rgb::new(244, 208, 92)),
                    };
                    blend_circle(fb, right - r, y, r, Rgb::new(14, 14, 16), 230);
                    fb.circle_outline(right - r, y, r, color);
                    let top = y - (EM_TAG * h * 0.50) as i32;
                    self.pad_label(fb, letter, right - r, top, EM_TAG, color, 0);
                    r * 2
                }
            };
            let top = y - (EM_LABEL * h * 0.50) as i32;
            self.pad_label(
                fb,
                label,
                right - glyph_w - (h * 0.010) as i32,
                top,
                EM_LABEL,
                Rgb::new(240, 236, 224),
                1,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_sit_on_the_clock_positions() {
        // Screen axes: +x right, +y down.  Slot 0 is twelve o'clock.
        assert_eq!(slot_at(0.0, -1.0), 0);
        assert_eq!(slot_at(1.0, 0.0), 3);
        assert_eq!(slot_at(0.0, 1.0), 6);
        assert_eq!(slot_at(-1.0, 0.0), 9);
        // A wedge is 30 degrees wide, centred on its hour.
        let deg = |d: f32| {
            let a = d.to_radians();
            slot_at(a.sin(), -a.cos())
        };
        assert_eq!(deg(14.0), 0);
        assert_eq!(deg(16.0), 1);
        assert_eq!(deg(346.0), 0);
        assert_eq!(deg(344.0), 11);
        let ring = Ring {
            cx: 100,
            cy: 100,
            outer: 50.0,
            inner: 30.0,
            dim: false,
        };
        assert_eq!(slot_centre(&ring, 0, 40.0), (100, 60));
        assert_eq!(slot_centre(&ring, 3, 40.0), (140, 100));
    }

    #[test]
    fn radial_keeps_the_reference_proportions() {
        // 1920x1080 reference: 432 px focused radial with a 288 px hole,
        // neighbours at 64 % of it.
        let h = 1080.0;
        assert_eq!((RADIAL_OUTER * h * 2.0).round(), 432.0);
        assert_eq!((RADIAL_INNER * h * 2.0).round(), 287.0);
        assert!((SIDE_OUTER / RADIAL_OUTER - 0.64).abs() < 0.01);
        assert_eq!(PadShortcut::ALL.len(), SLOTS);
    }

    #[test]
    fn ring_paints_the_focused_wedge_and_leaves_the_outside_alone() {
        let mut fb = Framebuffer::new(200, 200);
        fb.clear(Rgb::new(90, 90, 90));
        let ring = Ring {
            cx: 100,
            cy: 100,
            outer: 80.0,
            inner: 52.0,
            dim: false,
        };
        let mut filled = [false; SLOTS];
        filled[0] = true;
        filled[3] = true;
        draw_ring(&mut fb, &ring, &filled, Some(3));
        let rgb = |x, y| {
            let c = fb.get(x, y).0;
            ((c >> 16) & 255, (c >> 8) & 255, c & 255)
        };
        // Outside the ring nothing changes.
        assert_eq!(rgb(5, 5), (90, 90, 90));
        // Three o'clock is focused: rust orange (red dominates).
        let (r, g, _) = rgb(166, 100);
        assert!(r > 120 && r > g * 2, "focused wedge {r} {g}");
        // Twelve o'clock is a filled green wedge.
        let (r, g, _) = rgb(100, 34);
        assert!(g > r, "filled wedge {r} {g}");
        // The centre stays mostly see-through.
        let (r, _, _) = rgb(100, 100);
        assert!(r > 70, "centre {r}");
    }

    #[test]
    fn tooltips_wrap_on_words() {
        let lines = wrap_text("one two three four five six", 9, 3);
        assert_eq!(lines, vec!["one two", "three", "four five"]);
    }
}
