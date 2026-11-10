//! Gamepad support: one optional `gilrs` backend that turns a standard
//! controller into the same mouse/keyboard intent the client already
//! handles.  The simulation never sees a gamepad.
//!
//! Linux reads evdev through `gilrs` (uaccess/udev); Windows uses XInput,
//! macOS IOKit.  Set `SC_GAMEPAD=0` to disable the backend.

use gilrs::{Axis, Button, Gilrs};
use minifb::Key;

#[cfg(target_os = "linux")]
use std::io::Read;
#[cfg(target_os = "linux")]
use std::os::fd::AsRawFd;

/// Dead zone of both sticks (radial, in the -1..1 range).
pub const DEAD_ZONE: f32 = 0.18;
/// Virtual cursor speed at full left-stick deflection, window px/frame.
pub const CURSOR_SPEED: f32 = 16.0;
/// Camera pan at full right-stick deflection, game px/frame.
pub const PAN_SPEED: i32 = 14;
/// Right-stick deflection that starts moving the command-wheel focus.
pub const WHEEL_AIM_THRESHOLD: f32 = 0.5;
/// A second A press within this window selects every unit of the same type.
pub const DOUBLE_TAP_MS: u64 = 350;

/// One frame of controller intent.
#[derive(Clone, Debug, Default)]
pub struct PadFrame {
    pub active: bool,
    /// Cursor movement in window pixels this frame.
    pub cursor: (f32, f32),
    /// Camera pan (x, y) in game pixels this frame.
    pub pan: (i32, i32),
    /// Right stick after the radial dead zone, screen axes (+x right,
    /// +y down).  Drives the command wheel while it is open.
    pub aim: (f32, f32),
    /// Held trigger/button levels.
    pub lmb: bool,
    pub shift: bool,
    pub ctrl: bool,
    /// A edge: select / confirm / activate.
    pub primary: bool,
    /// B edge: cancel / back / deselect.
    pub cancel: bool,
    /// X edge: open the context command wheel.
    pub command_wheel: bool,
    /// Y edge: cycle an idle worker / army unit.
    pub cycle_idle: bool,
    /// LB / RB edges: previous / next command wheel page.
    pub page_prev: bool,
    pub page_next: bool,
    /// Start edge: game menu.
    pub menu: bool,
    /// Select/View edge: objectives / score.
    pub objectives: bool,
    /// Y edge: issue an attack-move to the pointer.
    pub attack_move: bool,
    /// X edge: smart order (move / attack / gather) at the pointer.
    pub smart_order: bool,
    /// RT held: the nine positional buttons address the command card.
    pub command_layer: bool,
    /// RT + one of the nine positional buttons: command-card index 0..8.
    pub command_slot: Option<usize>,
    /// LB + one of eight buttons: group edge / held slot (0..7 => groups 1..8).
    pub group_pressed: Option<usize>,
    pub group_held: Option<usize>,
    /// D-pad down/left/right without a modifier: saved camera 0..2.
    pub camera_slot: Option<usize>,
    /// LT + camera D-pad edge: save the current camera.
    pub camera_save: bool,
    /// LT + camera D-pad edge: slot 0..2 to overwrite.
    pub camera_save_slot: Option<usize>,
    /// D-pad up: jump to the last damage alert.
    pub last_alert: bool,
    /// RB held: confine the pointer to the minimap; A jumps there.
    pub minimap_hold: bool,
    /// Edge-triggered keys for the in-game handler.
    pub pressed: Vec<Key>,
    /// Edge-triggered keys for menus/dialogs (A is Enter, B is Escape).
    pub menu_pressed: Vec<Key>,
    /// Held arrow levels for menu navigation/camera.
    pub nav: (bool, bool, bool, bool),
    /// D-pad edges (left, right, up, down) for wheel navigation.
    pub nav_pressed: (bool, bool, bool, bool),
    pub wheel: i32,
}

/// Pure button bits, so the mapping can be tested without a device.
#[derive(Clone, Copy, Debug, Default)]
pub struct PadButtons {
    pub south: bool,
    pub east: bool,
    pub west: bool,
    pub north: bool,
    pub lb: bool,
    pub rb: bool,
    pub start: bool,
    pub select: bool,
    pub lt: bool,
    pub rt: bool,
    pub dpad_up: bool,
    pub dpad_down: bool,
    pub dpad_left: bool,
    pub dpad_right: bool,
    pub l3: bool,
    pub r3: bool,
}

fn radial(x: f32, y: f32) -> (f32, f32) {
    let len = (x * x + y * y).sqrt();
    if len <= DEAD_ZONE {
        return (0.0, 0.0);
    }
    let scaled = ((len - DEAD_ZONE) / (1.0 - DEAD_ZONE)).min(1.0);
    let norm = scaled / len;
    (x * norm, y * norm)
}

/// Map one state snapshot to a frame.  `prev` is the previous snapshot, so
/// button presses are edge-triggered exactly once.
pub fn map_state(
    left: (f32, f32),
    right: (f32, f32),
    buttons: PadButtons,
    prev: PadButtons,
    wheel: i32,
) -> PadFrame {
    let (lx, ly) = radial(left.0, left.1);
    let (rx, ry) = radial(right.0, right.1);
    let dpad_edge = |now: bool, before: bool| now && !before;
    let rt = buttons.rt;
    let lb = buttons.lb;
    let lt = buttons.lt;
    let edge = |now: bool, before: bool| now && !before;
    let group_button =
        |i: usize, now: PadButtons, before: PadButtons| -> (Option<usize>, Option<usize>) {
            let slot = match i {
                0 => (now.dpad_down, before.dpad_down, 0usize),
                1 => (now.dpad_left, before.dpad_left, 1),
                2 => (now.dpad_right, before.dpad_right, 2),
                3 => (now.dpad_up, before.dpad_up, 3),
                4 => (now.south, before.south, 4),
                5 => (now.east, before.east, 5),
                6 => (now.west, before.west, 6),
                _ => (now.north, before.north, 7),
            };
            if lb && slot.0 {
                (edge(slot.0, slot.1).then_some(slot.2), Some(slot.2))
            } else {
                (None, None)
            }
        };
    let mut group_pressed = None;
    let mut group_held = None;
    for i in 0..8 {
        let (pressed, held) = group_button(i, buttons, prev);
        if pressed.is_some() {
            group_pressed = pressed;
        }
        if held.is_some() {
            group_held = held;
        }
    }
    let command_slot = if rt {
        let slots = [
            (buttons.west, prev.west),
            (buttons.north, prev.north),
            (buttons.east, prev.east),
            (buttons.dpad_left, prev.dpad_left),
            (buttons.dpad_up, prev.dpad_up),
            (buttons.dpad_right, prev.dpad_right),
            (buttons.dpad_down, prev.dpad_down),
            (buttons.south, prev.south),
            (buttons.rb, prev.rb),
        ];
        slots.iter().position(|&(now, before)| edge(now, before))
    } else {
        None
    };
    let camera_modifier = !lb && !rt;
    let camera_slot = if camera_modifier && !lt {
        if edge(buttons.dpad_down, prev.dpad_down) {
            Some(0)
        } else if edge(buttons.dpad_left, prev.dpad_left) {
            Some(1)
        } else if edge(buttons.dpad_right, prev.dpad_right) {
            Some(2)
        } else {
            None
        }
    } else {
        None
    };
    let camera_save_slot = if camera_modifier && lt {
        if edge(buttons.dpad_down, prev.dpad_down) {
            Some(0)
        } else if edge(buttons.dpad_left, prev.dpad_left) {
            Some(1)
        } else if edge(buttons.dpad_right, prev.dpad_right) {
            Some(2)
        } else {
            None
        }
    } else {
        None
    };
    let camera_save = camera_save_slot.is_some();
    let last_alert = camera_modifier && !lt && edge(buttons.dpad_up, prev.dpad_up);
    let mut frame = PadFrame {
        active: lx != 0.0
            || ly != 0.0
            || rx != 0.0
            || ry != 0.0
            || buttons.south
            || buttons.east
            || buttons.west
            || buttons.north
            || buttons.lb
            || buttons.rb
            || buttons.start
            || buttons.select
            || buttons.lt
            || buttons.rt
            || buttons.dpad_up
            || buttons.dpad_down
            || buttons.dpad_left
            || buttons.dpad_right
            || buttons.l3
            || buttons.r3,
        cursor: (lx * CURSOR_SPEED, -ly * CURSOR_SPEED),
        pan: (
            (rx * PAN_SPEED as f32).round() as i32,
            (-ry * PAN_SPEED as f32).round() as i32,
        ),
        aim: (rx, -ry),
        lmb: buttons.south,
        shift: lt,
        ctrl: false,
        attack_move: !rt && !lb && edge(buttons.north, prev.north),
        smart_order: !rt && !lb && edge(buttons.west, prev.west),
        command_layer: rt,
        command_slot,
        group_pressed,
        group_held,
        camera_slot,
        camera_save,
        camera_save_slot,
        last_alert,
        minimap_hold: buttons.rb && !rt,
        primary: buttons.south && !prev.south && !rt && !lb,
        cancel: buttons.east && !prev.east && !rt && !lb,
        command_wheel: false,
        cycle_idle: false,
        page_prev: false,
        page_next: false,
        menu: buttons.start && !prev.start,
        objectives: buttons.select && !prev.select,
        nav: (false, false, false, false),
        nav_pressed: (
            dpad_edge(buttons.dpad_left, prev.dpad_left) && !lb && !rt,
            dpad_edge(buttons.dpad_right, prev.dpad_right) && !lb && !rt,
            dpad_edge(buttons.dpad_up, prev.dpad_up) && !lb && !rt,
            dpad_edge(buttons.dpad_down, prev.dpad_down) && !lb && !rt,
        ),
        wheel,
        ..PadFrame::default()
    };
    // L3 remains a convenience for centring on the base.
    if buttons.l3 && !prev.l3 {
        frame.pressed.push(Key::Home);
    }
    // Menus: A activates, B goes back, Start confirms/opens the game menu.
    if frame.primary {
        frame.menu_pressed.push(Key::Enter);
    }
    if frame.cancel {
        frame.menu_pressed.push(Key::Escape);
    }
    if frame.menu {
        frame.menu_pressed.push(Key::F10);
        frame.menu_pressed.push(Key::Enter);
    }
    if frame.nav_pressed.0 {
        frame.menu_pressed.push(Key::Left);
    }
    if frame.nav_pressed.1 {
        frame.menu_pressed.push(Key::Right);
    }
    if frame.nav_pressed.2 {
        frame.menu_pressed.push(Key::Up);
    }
    if frame.nav_pressed.3 {
        frame.menu_pressed.push(Key::Down);
    }
    frame
}

/// Controller command-wheel state: which page and sector are focused.  The
/// caller supplies the page sizes, so the state is UI-independent and can be
/// tested headlessly.  Pages with no entries are dropped.
#[derive(Clone, Debug, Default)]
pub struct WheelState {
    open: bool,
    page: usize,
    focus: usize,
    counts: Vec<usize>,
}

impl WheelState {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn page(&self) -> usize {
        self.page
    }

    pub fn focus(&self) -> usize {
        self.focus
    }

    pub fn pages(&self) -> usize {
        self.counts.len()
    }

    /// Open the wheel with one entry count per page.  Returns false (and
    /// stays closed) when every page is empty.
    pub fn open<I: IntoIterator<Item = usize>>(&mut self, counts: I) -> bool {
        let counts: Vec<usize> = counts.into_iter().filter(|&n| n > 0).collect();
        if counts.is_empty() {
            self.close();
            return false;
        }
        self.counts = counts;
        self.page = 0;
        self.focus = 0;
        self.open = true;
        true
    }

    pub fn close(&mut self) {
        self.open = false;
        self.page = 0;
        self.focus = 0;
        self.counts.clear();
    }

    /// LB / RB: switch to the previous or next non-empty page.
    pub fn page_delta(&mut self, delta: isize) -> bool {
        if !self.open || self.counts.len() <= 1 {
            return false;
        }
        let len = self.counts.len() as isize;
        let next = (self.page as isize + delta).rem_euclid(len) as usize;
        if next == self.page {
            return false;
        }
        self.page = next;
        self.focus = 0;
        true
    }

    /// D-pad: move the focused sector, wrapping around.
    pub fn focus_delta(&mut self, delta: isize) -> bool {
        if !self.open {
            return false;
        }
        let len = self.counts.get(self.page).copied().unwrap_or(0);
        if len == 0 {
            return false;
        }
        let next = (self.focus as isize + delta).rem_euclid(len as isize) as usize;
        let changed = next != self.focus;
        self.focus = next;
        changed
    }

    /// Right stick: focus the sector under the stick.  `dx`/`dy` are screen
    /// axes (right / down) and index 0 is the top sector, clockwise.
    pub fn aim(&mut self, dx: f32, dy: f32) -> bool {
        if !self.open || (dx * dx + dy * dy).sqrt() < WHEEL_AIM_THRESHOLD {
            return false;
        }
        let len = self.counts.get(self.page).copied().unwrap_or(0);
        if len == 0 {
            return false;
        }
        let turns = (dy.atan2(dx) + std::f32::consts::FRAC_PI_2).rem_euclid(std::f32::consts::TAU)
            / std::f32::consts::TAU;
        let next = ((turns * len as f32).round() as usize) % len;
        let changed = next != self.focus;
        self.focus = next;
        changed
    }

    /// A: return the focused `(page, sector)` and close the wheel.
    pub fn activate(&mut self) -> Option<(usize, usize)> {
        if !self.open {
            return None;
        }
        let picked = (self.page, self.focus);
        self.close();
        Some(picked)
    }
}

/// Two taps on the same unit type within [`DOUBLE_TAP_MS`] select every
/// visible unit of that type.  Time is supplied by the caller so the state is
/// deterministic in tests.
#[derive(Clone, Debug, Default)]
pub struct TapTracker {
    last: Option<(u64, u32)>,
}

impl TapTracker {
    pub fn tap(&mut self, now_ms: u64, key: u32) -> bool {
        let double = self
            .last
            .is_some_and(|(t, k)| k == key && now_ms.saturating_sub(t) <= DOUBLE_TAP_MS);
        self.last = if double { None } else { Some((now_ms, key)) };
        double
    }

    pub fn reset(&mut self) {
        self.last = None;
    }
}

/// drivers publish a usable js device while gilrs' evdev snapshot remains
/// neutral; the kernel joystick state is then the authoritative one.
#[cfg(target_os = "linux")]
struct JsPad {
    file: std::fs::File,
    /// ABS code -> value, indexed by the kernel event-code number.
    axes: [f32; 64],
    /// KEY code -> held, indexed by the kernel key code.
    buttons: [bool; 0x300],
    /// JS axis index -> ABS code (`JSIOCGAXMAP`).
    axis_map: [u8; 64],
    /// JS button index -> KEY code (`JSIOCGBTNMAP`).
    button_map: [u16; 0x200],
}

#[cfg(target_os = "linux")]
impl JsPad {
    fn open_all() -> Vec<Self> {
        let mut out = Vec::new();
        for n in 0..4 {
            let path = format!("/dev/input/js{n}");
            let Ok(file) = std::fs::OpenOptions::new().read(true).open(&path) else {
                continue;
            };
            unsafe {
                let fd = file.as_raw_fd();
                let flags = libc::fcntl(fd, libc::F_GETFL);
                libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
            }
            let mut axis_map = [0u8; 64];
            let mut button_map = [0u16; 0x200];
            unsafe {
                let fd = file.as_raw_fd();
                libc::ioctl(fd, 0x8040_6a32, axis_map.as_mut_ptr());
                libc::ioctl(fd, 0x8400_6a34, button_map.as_mut_ptr());
            }
            out.push(Self {
                file,
                axes: [0.0; 64],
                buttons: [false; 0x300],
                axis_map,
                button_map,
            });
        }
        out
    }

    /// Drain pending 8-byte `js_event` records and return the current state.
    fn poll(&mut self) -> Option<(f32, f32, f32, f32, PadButtons)> {
        let mut buf = [0u8; 8];
        loop {
            match self.file.read(&mut buf) {
                Ok(8) => {
                    let value = i16::from_le_bytes([buf[4], buf[5]]);
                    let kind = buf[6];
                    let number = buf[7] as usize;
                    match kind & 0x7f {
                        1 => {
                            let code = self.button_map.get(number).copied().unwrap_or(0) as usize;
                            if let Some(slot) = self.buttons.get_mut(code) {
                                *slot = value != 0;
                            }
                        }
                        2 => {
                            let code = self.axis_map.get(number).copied().unwrap_or(0) as usize;
                            if let Some(slot) = self.axes.get_mut(code) {
                                *slot = value as f32 / 32767.0;
                            }
                        }
                        _ => {}
                    }
                }
                Ok(_) => break,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => return None,
            }
        }
        let axis = |i: usize| self.axes.get(i).copied().unwrap_or(0.0);
        let pressed = |i: usize| self.buttons.get(i).copied().unwrap_or(false);
        let hat_x = axis(16);
        let hat_y = axis(17);
        // Kernel js trigger axes are either signed (-1..1) or 8-bit
        // (0..255, hence 0..0.0078 after the /32767 normalisation above).
        let trigger = |code: usize, button: usize| {
            let level = axis(code);
            pressed(button) || level > 0.5 || (level > 0.0 && level < 0.02)
        };
        let buttons = PadButtons {
            south: pressed(0x130),
            east: pressed(0x131),
            west: pressed(0x134),
            north: pressed(0x133),
            lb: pressed(0x136),
            rb: pressed(0x137),
            start: pressed(0x13b),
            select: pressed(0x13a),
            lt: trigger(2, 0x138),
            rt: trigger(5, 0x139),
            dpad_up: pressed(0x220) || hat_y < -0.5,
            dpad_down: pressed(0x221) || hat_y > 0.5,
            dpad_left: pressed(0x222) || hat_x < -0.5,
            dpad_right: pressed(0x223) || hat_x > 0.5,
            l3: pressed(0x13d),
            r3: pressed(0x13e),
        };
        Some((axis(0), -axis(1), axis(3), -axis(4), buttons))
    }

    fn describe(&self) -> String {
        format!(
            "js left=({:.2},{:.2}) right=({:.2},{:.2})",
            self.axes[0], -self.axes[1], self.axes[3], -self.axes[4]
        )
    }
}

/// The live controller backend.  `None` when no controller/system backend is
/// available; the rest of the client keeps working.
pub struct Gamepad {
    gilrs: Option<Gilrs>,
    #[cfg(target_os = "linux")]
    js: Vec<JsPad>,
    prev: PadButtons,
    wheel: i32,
}

impl Gamepad {
    pub fn new() -> Self {
        if std::env::var("SC_GAMEPAD").ok().as_deref() == Some("0") {
            return Self {
                gilrs: None,
                #[cfg(target_os = "linux")]
                js: Vec::new(),
                prev: PadButtons::default(),
                wheel: 0,
            };
        }
        let gilrs = Gilrs::new().ok();
        Self {
            gilrs,
            #[cfg(target_os = "linux")]
            js: JsPad::open_all(),
            prev: PadButtons::default(),
            wheel: 0,
        }
    }

    /// Backend and pad description for diagnostics/acceptance probes.
    pub fn describe(&self) -> String {
        #[cfg(target_os = "linux")]
        let js = if self.js.is_empty() {
            "js unavailable".to_string()
        } else {
            self.js
                .iter()
                .map(JsPad::describe)
                .collect::<Vec<_>>()
                .join(" | ")
        };
        let Some(gilrs) = self.gilrs.as_ref() else {
            #[cfg(target_os = "linux")]
            return format!("gilrs backend unavailable; {js}");
            #[cfg(not(target_os = "linux"))]
            return "gilrs backend unavailable".to_string();
        };
        let pads: Vec<String> = gilrs
            .gamepads()
            .map(|(id, pad)| {
                format!(
                    "{id:?} {} left=({:.2},{:.2}) right=({:.2},{:.2})",
                    pad.name(),
                    pad.value(Axis::LeftStickX),
                    pad.value(Axis::LeftStickY),
                    pad.value(Axis::RightStickX),
                    pad.value(Axis::RightStickY)
                )
            })
            .collect();
        let gilrs = if pads.is_empty() {
            "gilrs: 0 gamepads".to_string()
        } else {
            format!("gilrs: {}", pads.join(" | "))
        };
        #[cfg(target_os = "linux")]
        return format!("{js}; {gilrs}");
        #[cfg(not(target_os = "linux"))]
        gilrs
    }

    pub fn poll(&mut self) -> PadFrame {
        #[cfg(target_os = "linux")]
        for js in self.js.iter_mut().rev() {
            if let Some((lx, ly, rx, ry, buttons)) = js.poll() {
                let frame = map_state((lx, ly), (rx, ry), buttons, self.prev, self.wheel);
                // A js device with a non-neutral snapshot is authoritative;
                // otherwise fall through to the next js/gilrs device.
                if frame.active {
                    self.prev = buttons;
                    self.wheel = 0;
                    return frame;
                }
            }
        }
        let Some(gilrs) = self.gilrs.as_mut() else {
            return PadFrame::default();
        };
        // Drain events so connection changes and button edges are not
        // missed; the state query below is the authoritative snapshot.
        while gilrs.next_event().is_some() {}
        let Some((_, pad)) = gilrs.gamepads().next() else {
            return PadFrame::default();
        };
        let axis = |a: Axis| pad.value(a);
        let buttons = PadButtons {
            south: pad.is_pressed(Button::South),
            east: pad.is_pressed(Button::East),
            west: pad.is_pressed(Button::West),
            north: pad.is_pressed(Button::North),
            lb: pad.is_pressed(Button::LeftTrigger),
            rb: pad.is_pressed(Button::RightTrigger),
            start: pad.is_pressed(Button::Start),
            select: pad.is_pressed(Button::Select),
            lt: pad.is_pressed(Button::LeftTrigger2) || pad.value(Axis::LeftZ) > 0.5,
            rt: pad.is_pressed(Button::RightTrigger2) || pad.value(Axis::RightZ) > 0.5,
            dpad_up: pad.is_pressed(Button::DPadUp),
            dpad_down: pad.is_pressed(Button::DPadDown),
            dpad_left: pad.is_pressed(Button::DPadLeft),
            dpad_right: pad.is_pressed(Button::DPadRight),
            l3: pad.is_pressed(Button::LeftThumb),
            r3: pad.is_pressed(Button::RightThumb),
        };
        let frame = map_state(
            (axis(Axis::LeftStickX), axis(Axis::LeftStickY)),
            (axis(Axis::RightStickX), axis(Axis::RightStickY)),
            buttons,
            self.prev,
            self.wheel,
        );
        self.prev = buttons;
        self.wheel = 0;
        frame
    }
}

impl Default for Gamepad {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dead_zone_is_radial_and_a_b_x_y_are_edges() {
        let still = map_state(
            (0.1, 0.1),
            (0.0, 0.0),
            PadButtons::default(),
            PadButtons::default(),
            0,
        );
        assert_eq!(still.cursor, (0.0, 0.0));
        assert!(!still.active);
        let b = PadButtons {
            south: true,
            west: true,
            dpad_left: true,
            lt: true,
            ..PadButtons::default()
        };
        let f = map_state((1.0, 0.0), (0.0, 0.0), b, PadButtons::default(), 0);
        assert!(f.active);
        assert!(f.lmb, "A still clicks the pointer in menus");
        assert!(f.primary);
        assert!(f.smart_order);
        assert!(f.shift, "LT is the queue/add modifier");
        assert!(f.menu_pressed.contains(&Key::Enter));
        assert!(!f.nav.0, "the D-pad is reserved for saved cameras");
        assert!(f.nav_pressed.0, "menus still see the D-pad edge");
        assert!(f.camera_save);
        assert_eq!(f.camera_save_slot, Some(1));
        assert!(f.cursor.0 > 0.0);
        // A held button does not press again.
        let f2 = map_state((1.0, 0.0), (0.0, 0.0), b, b, 0);
        assert!(!f2.primary);
        assert!(!f2.smart_order);
        assert!(!f2.menu_pressed.contains(&Key::Enter));
        assert!(!f2.nav_pressed.0);
    }

    #[test]
    fn controller_buttons_follow_the_starcraft_xbox_scheme() {
        let b = PadButtons {
            south: true,
            east: true,
            west: true,
            north: true,
            rb: true,
            start: true,
            select: true,
            ..PadButtons::default()
        };
        let f = map_state((0.0, 0.0), (0.0, 0.0), b, PadButtons::default(), 0);
        assert!(f.primary, "A selects");
        assert!(f.cancel, "B cancels");
        assert!(f.smart_order, "X is the smart order");
        assert!(f.attack_move, "Y is attack-move");
        assert!(f.menu);
        assert!(f.objectives);
        assert!(f.minimap_hold, "RB confines the pointer to the minimap");
        assert!(f.menu_pressed.contains(&Key::Escape));
        assert!(f.menu_pressed.contains(&Key::F10));
        assert!(!f.command_wheel && !f.cycle_idle && !f.page_prev && !f.page_next);

        // RT turns the nine positional buttons into command-card slots in
        // exactly this order: X/Y/B, D-pad left/up/right, D-pad down/A/RB.
        let rt = PadButtons {
            rt: true,
            ..PadButtons::default()
        };
        let cases = [
            (
                "RT+X",
                PadButtons {
                    west: true,
                    ..PadButtons::default()
                },
                0,
            ),
            (
                "RT+Y",
                PadButtons {
                    north: true,
                    ..PadButtons::default()
                },
                1,
            ),
            (
                "RT+B",
                PadButtons {
                    east: true,
                    ..PadButtons::default()
                },
                2,
            ),
            (
                "RT+D-pad left",
                PadButtons {
                    dpad_left: true,
                    ..PadButtons::default()
                },
                3,
            ),
            (
                "RT+D-pad up",
                PadButtons {
                    dpad_up: true,
                    ..PadButtons::default()
                },
                4,
            ),
            (
                "RT+D-pad right",
                PadButtons {
                    dpad_right: true,
                    ..PadButtons::default()
                },
                5,
            ),
            (
                "RT+D-pad down",
                PadButtons {
                    dpad_down: true,
                    ..PadButtons::default()
                },
                6,
            ),
            (
                "RT+A",
                PadButtons {
                    south: true,
                    ..PadButtons::default()
                },
                7,
            ),
            (
                "RT+RB",
                PadButtons {
                    rb: true,
                    ..PadButtons::default()
                },
                8,
            ),
        ];
        for (name, pressed, want) in cases {
            let card = map_state(
                (0.0, 0.0),
                (0.0, 0.0),
                PadButtons {
                    rt: true,
                    ..pressed
                },
                rt,
                0,
            );
            assert_eq!(card.command_slot, Some(want), "{name}");
            assert!(card.command_layer, "{name} opens the card layer");
            assert!(!card.primary && !card.cancel && !card.attack_move, "{name}");
        }
    }

    #[test]
    fn lb_groups_and_dpad_cameras_are_positional() {
        // LB + A is group 5 in the 1..8 table.
        let lb = PadButtons {
            lb: true,
            ..PadButtons::default()
        };
        let f = map_state(
            (0.0, 0.0),
            (0.0, 0.0),
            PadButtons {
                lb: true,
                south: true,
                ..PadButtons::default()
            },
            lb,
            0,
        );
        assert_eq!(f.group_pressed, Some(4));
        assert_eq!(f.group_held, Some(4));
        assert!(
            !f.primary,
            "LB+A does not select while the group layer is held"
        );
        // LB + D-pad down is group 1.
        let f = map_state(
            (0.0, 0.0),
            (0.0, 0.0),
            PadButtons {
                lb: true,
                dpad_down: true,
                ..PadButtons::default()
            },
            lb,
            0,
        );
        assert_eq!(f.group_pressed, Some(0));
        // D-pad down restores camera 1, LT+D-pad down saves it.
        let f = map_state(
            (0.0, 0.0),
            (0.0, 0.0),
            PadButtons {
                dpad_down: true,
                ..PadButtons::default()
            },
            PadButtons::default(),
            0,
        );
        assert_eq!(f.camera_slot, Some(0));
        assert!(!f.camera_save);
        let f = map_state(
            (0.0, 0.0),
            (0.0, 0.0),
            PadButtons {
                lt: true,
                dpad_left: true,
                ..PadButtons::default()
            },
            PadButtons {
                lt: true,
                ..PadButtons::default()
            },
            0,
        );
        assert!(f.camera_save);
        assert_eq!(f.camera_slot, None);
        // D-pad up is the last-alert jump.
        let f = map_state(
            (0.0, 0.0),
            (0.0, 0.0),
            PadButtons {
                dpad_up: true,
                ..PadButtons::default()
            },
            PadButtons::default(),
            0,
        );
        assert!(f.last_alert);
    }

    #[test]
    fn right_stick_pans_at_the_camera_speed_and_aims_the_wheel() {
        let f = map_state(
            (0.0, 0.0),
            (1.0, 0.0),
            PadButtons::default(),
            PadButtons::default(),
            0,
        );
        assert_eq!(f.pan, (PAN_SPEED, 0));
        assert_eq!(f.aim, (1.0, 0.0));
        let up = map_state(
            (0.0, 0.0),
            (0.0, 1.0),
            PadButtons::default(),
            PadButtons::default(),
            0,
        );
        assert_eq!(up.pan, (0, -PAN_SPEED));
        assert_eq!(up.aim, (0.0, -1.0));
    }

    #[test]
    fn wheel_opens_closes_and_pages_skip_empty_categories() {
        let mut wheel = WheelState::default();
        assert!(!wheel.is_open());
        assert!(!wheel.open([]));
        assert!(!wheel.is_open());
        assert!(wheel.open([3, 0, 5]));
        assert_eq!(wheel.pages(), 2, "empty pages are dropped");
        assert_eq!((wheel.page(), wheel.focus()), (0, 0));
        assert!(wheel.focus_delta(1));
        assert_eq!(wheel.focus(), 1);
        assert!(wheel.focus_delta(1));
        assert_eq!(wheel.focus(), 2);
        assert!(wheel.focus_delta(1));
        assert_eq!(wheel.focus(), 0, "focus wraps");
        assert!(wheel.page_delta(1));
        assert_eq!((wheel.page(), wheel.focus()), (1, 0));
        assert!(wheel.focus_delta(-1));
        assert_eq!(wheel.focus(), 4, "previous from the top wraps");
        assert_eq!(wheel.activate(), Some((1, 4)));
        assert!(!wheel.is_open());
        assert_eq!(wheel.activate(), None);
    }

    #[test]
    fn wheel_aim_picks_the_sector_under_the_right_stick() {
        let mut wheel = WheelState::default();
        assert!(wheel.open([4]));
        assert!(!wheel.aim(0.0, 0.0), "inside the aim dead zone");
        assert!(wheel.aim(1.0, 0.0));
        assert_eq!(wheel.focus(), 1, "right sector, clockwise");
        assert!(wheel.aim(0.0, 1.0));
        assert_eq!(wheel.focus(), 2, "bottom sector");
        assert!(wheel.aim(-1.0, 0.0));
        assert_eq!(wheel.focus(), 3, "left sector");
        assert!(wheel.aim(0.0, -1.0));
        assert_eq!(wheel.focus(), 0, "top sector");
        assert!(
            !wheel.aim(0.0, -1.0),
            "describes the already focused sector"
        );
        let mut one = WheelState::default();
        assert!(one.open([1]));
        assert!(!one.aim(1.0, 0.0), "a one-entry page never changes");
    }

    #[test]
    fn double_tap_tracker_matches_the_same_type_within_the_window() {
        let mut taps = TapTracker::default();
        assert!(!taps.tap(1_000, 7));
        assert!(!taps.tap(1_100, 8), "different type");
        assert!(taps.tap(1_300, 8), "same type within 350 ms");
        assert!(!taps.tap(1_450, 8), "a third tap starts a new pair");
        assert!(taps.tap(1_600, 8), "the pair is complete");
        taps.reset();
        assert!(!taps.tap(1_700, 8));
        assert!(!taps.tap(2_200, 8), "window expired");
    }
}
