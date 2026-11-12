//! Software renderer for the reconstructed rules engine.
//!
//! This module is deliberately dependency-free apart from the text glyphs:
//! it decodes the retail GRP sprites and the CV5/VX4EX/VR4/WPE tileset chain
//! and draws them into a 32-bit RGB framebuffer.  `sc_view` presents that
//! framebuffer with `minifb`; `sc_shot` writes it to an image for headless
//! verification.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use font8x8::UnicodeFonts;

use crate::data::GameData;
use crate::ids::unit as uid;
use crate::map::Scenario;
use crate::sim::SimUnit;

pub const TILE_PX: i32 = 32;
pub const MINITILE_PX: i32 = 8;

/// ARGB pixel used by `minifb` (`0x00RRGGBB`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rgb(pub u32);

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self(((r as u32) << 16) | ((g as u32) << 8) | b as u32)
    }
    pub fn blend(self, other: Rgb, alpha: u8) -> Rgb {
        let a = alpha as u32;
        let inv = 255 - a;
        let r = (((self.0 >> 16) & 0xff) * inv + ((other.0 >> 16) & 0xff) * a) / 255;
        let g = (((self.0 >> 8) & 0xff) * inv + ((other.0 >> 8) & 0xff) * a) / 255;
        let b = ((self.0 & 0xff) * inv + (other.0 & 0xff) * a) / 255;
        Rgb((r << 16) | (g << 8) | b)
    }
}

/// Pixels saved by [`Framebuffer::save_rect`].
pub struct SavedRect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    pixels: Vec<u32>,
}

/// How much of a cloaked unit shows through for players who can see it
/// (its owner, or an enemy with a detector).  INFERENCE: retail draws
/// cloaked units with a palette distortion effect; this true-colour view
/// fades them instead.
pub const CLOAK_KEEP: u8 = 110;

/// Colour `owner`'s units, buildings and minimap blips are drawn in: its
/// lobby colour, else the classic slot colour.
pub fn player_rgb(world: &crate::sim::World, owner: u8) -> Rgb {
    match world.players.get(owner as usize).and_then(|p| p.color) {
        Some(c) => crate::multiplayer::color_rgb(c as usize),
        None => PLAYER_COLORS[owner as usize % PLAYER_COLORS.len()],
    }
}

pub const PLAYER_COLORS: [Rgb; 12] = [
    Rgb::new(255, 35, 35),
    Rgb::new(0, 80, 255),
    Rgb::new(0, 200, 200),
    Rgb::new(160, 60, 220),
    Rgb::new(255, 140, 0),
    Rgb::new(110, 70, 40),
    Rgb::new(230, 230, 230),
    Rgb::new(230, 220, 40),
    Rgb::new(40, 180, 70),
    Rgb::new(220, 110, 160),
    Rgb::new(120, 120, 255),
    Rgb::new(180, 180, 120),
];

/// Run `f(first_row, rows)` over horizontal bands of `pixels` (`stride`
/// pixels per row) on up to 8 threads.  Each band is a disjoint run of
/// whole rows, so per-row passes (fog, column stretch, terrain) need no
/// synchronisation; small images run on the calling thread.
pub fn par_bands<F>(pixels: &mut [u32], stride: usize, f: F)
where
    F: Fn(usize, &mut [u32]) + Sync,
{
    let rows = pixels.len() / stride.max(1);
    let threads = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .clamp(1, 8)
        .min(rows / 64)
        .max(1);
    if threads == 1 {
        f(0, pixels);
        return;
    }
    let per = rows.div_ceil(threads);
    std::thread::scope(|scope| {
        for (k, band) in pixels.chunks_mut(per * stride).enumerate() {
            let f = &f;
            scope.spawn(move || f(k * per, band));
        }
    });
}

pub struct Framebuffer {
    pub width: i32,
    pub height: i32,
    pub pixels: Vec<u32>,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width: width as i32,
            height: height as i32,
            pixels: vec![0; width * height],
        }
    }

    pub fn clear(&mut self, color: Rgb) {
        self.pixels.fill(color.0);
    }

    #[inline]
    pub fn put(&mut self, x: i32, y: i32, color: Rgb) {
        if x >= 0 && y >= 0 && x < self.width && y < self.height {
            self.pixels[y as usize * self.width as usize + x as usize] = color.0;
        }
    }

    /// Copy of the pixels of a rectangle (clipped), for [`Self::fade_rect`].
    pub fn save_rect(&self, x: i32, y: i32, w: i32, h: i32) -> SavedRect {
        let x0 = x.clamp(0, self.width);
        let y0 = y.clamp(0, self.height);
        let x1 = (x + w).clamp(0, self.width);
        let y1 = (y + h).clamp(0, self.height);
        let mut pixels = Vec::with_capacity(((x1 - x0) * (y1 - y0)).max(0) as usize);
        for yy in y0..y1 {
            let row = yy as usize * self.width as usize;
            pixels.extend_from_slice(&self.pixels[row + x0 as usize..row + x1 as usize]);
        }
        SavedRect {
            x: x0,
            y: y0,
            w: x1 - x0,
            h: y1 - y0,
            pixels,
        }
    }

    /// Blend what was drawn over a saved rectangle back towards the saved
    /// pixels: `keep` (0..=255) is how much of the new drawing remains.  A
    /// cloaked unit is drawn normally and then faded this way.
    pub fn fade_rect(&mut self, saved: &SavedRect, keep: u8) {
        for yy in 0..saved.h {
            let row = (saved.y + yy) as usize * self.width as usize + saved.x as usize;
            for xx in 0..saved.w {
                let old = Rgb(saved.pixels[(yy * saved.w + xx) as usize]);
                let new = Rgb(self.pixels[row + xx as usize]);
                if old.0 != new.0 {
                    self.pixels[row + xx as usize] = old.blend(new, keep).0;
                }
            }
        }
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> Rgb {
        if x >= 0 && y >= 0 && x < self.width && y < self.height {
            Rgb(self.pixels[y as usize * self.width as usize + x as usize])
        } else {
            Rgb(0)
        }
    }

    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, color: Rgb) {
        for yy in y..y + h {
            for xx in x..x + w {
                self.put(xx, yy, color);
            }
        }
    }

    pub fn rect_outline(&mut self, x: i32, y: i32, w: i32, h: i32, color: Rgb) {
        for xx in x..x + w {
            self.put(xx, y, color);
            self.put(xx, y + h - 1, color);
        }
        for yy in y..y + h {
            self.put(x, yy, color);
            self.put(x + w - 1, yy, color);
        }
    }

    pub fn line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Rgb) {
        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;
        let (mut x, mut y) = (x0, y0);
        loop {
            self.put(x, y, color);
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
    }

    /// Filled diamond, used for the mineral HUD icon.
    pub fn diamond_icon(&mut self, x: i32, y: i32, size: i32, color: Rgb) {
        let s = size.max(2);
        let half = s / 2;
        for dy in 0..s {
            let dist = (dy - half).abs();
            let w = half - dist / 2;
            for dx in -w..=w {
                self.put(x + half + dx, y + dy, color);
            }
        }
    }

    /// Filled up-chevron, used for the supply HUD icon.
    pub fn triangle_icon(&mut self, x: i32, y: i32, size: i32, color: Rgb) {
        let s = size.max(2);
        for dy in 0..s {
            let w = (s - dy) / 2;
            for dx in -w..=w {
                self.put(x + s / 2 + dx, y + dy, color);
            }
        }
    }

    pub fn circle_outline(&mut self, cx: i32, cy: i32, radius: i32, color: Rgb) {
        let mut x = radius;
        let mut y = 0;
        let mut err = 1 - radius;
        while x >= y {
            for (px, py) in [
                (cx + x, cy + y),
                (cx + y, cy + x),
                (cx - y, cy + x),
                (cx - x, cy + y),
                (cx - x, cy - y),
                (cx - y, cy - x),
                (cx + y, cy - x),
                (cx + x, cy - y),
            ] {
                self.put(px, py, color);
            }
            y += 1;
            if err < 0 {
                err += 2 * y + 1;
            } else {
                x -= 1;
                err += 2 * (y - x) + 1;
            }
        }
    }

    pub fn text(&mut self, x: i32, y: i32, text: &str, color: Rgb, scale: i32) {
        let mut cx = x;
        for ch in text.chars() {
            if ch == '\n' {
                cx = x;
                continue;
            }
            if let Some(glyph) = font8x8::BASIC_FONTS.get(ch) {
                for (row, bits) in glyph.iter().enumerate() {
                    for col in 0..8 {
                        if bits & (1 << col) != 0 {
                            self.rect(
                                cx + col * scale,
                                y + row as i32 * scale,
                                scale,
                                scale,
                                color,
                            );
                        }
                    }
                }
            }
            cx += 8 * scale;
        }
    }

    pub fn text_centered(&mut self, cx: i32, y: i32, text: &str, color: Rgb, scale: i32) {
        let w = text.chars().count() as i32 * 8 * scale;
        self.text(cx - w / 2, y, text, color, scale);
    }

    pub fn to_rgb_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.pixels.len() * 3);
        for p in &self.pixels {
            out.push(((p >> 16) & 0xff) as u8);
            out.push(((p >> 8) & 0xff) as u8);
            out.push((p & 0xff) as u8);
        }
        out
    }

    pub fn write_ppm(&self, path: &Path) -> std::io::Result<()> {
        let mut out = format!("P6\n{} {}\n255\n", self.width, self.height).into_bytes();
        out.extend_from_slice(&self.to_rgb_bytes());
        fs::write(path, out)
    }

    pub fn write_bmp(&self, path: &Path) -> std::io::Result<()> {
        let row = ((self.width * 3 + 3) / 4 * 4) as usize;
        let size = 54 + row * self.height as usize;
        let mut out = Vec::with_capacity(size);
        out.extend_from_slice(b"BM");
        out.extend_from_slice(&(size as u32).to_le_bytes());
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&54u32.to_le_bytes());
        out.extend_from_slice(&40u32.to_le_bytes());
        out.extend_from_slice(&self.width.to_le_bytes());
        out.extend_from_slice(&self.height.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&24u16.to_le_bytes());
        out.extend_from_slice(&[0; 24]);
        for y in (0..self.height).rev() {
            let mut line = vec![0u8; row];
            for x in 0..self.width {
                let p = self.get(x, y);
                let b = x as usize * 3;
                line[b] = (p.0 & 0xff) as u8;
                line[b + 1] = ((p.0 >> 8) & 0xff) as u8;
                line[b + 2] = ((p.0 >> 16) & 0xff) as u8;
            }
            out.extend_from_slice(&line);
        }
        fs::write(path, out)
    }
}

/// 256-colour palette.  WPE files are stored as RGBX; `Units.pal` is RGB.
#[derive(Clone)]
pub struct Palette {
    pub colors: [Rgb; 256],
}

/// Palette ranges the retail engine cycles for each tileset.  From the
/// classic rotator sets: badlands/jungle have three rotators, ashworld
/// three, desert/ice/twilight two and platform/installation none.
fn color_cycle_ranges(name: &str) -> &'static [(usize, usize)] {
    match name {
        "badlands" | "jungle" => &[(1, 6), (7, 13), (248, 254)],
        "ashworld" => &[(1, 4), (5, 8), (9, 13)],
        "desert" | "ice" | "twilight" => &[(1, 13), (248, 254)],
        _ => &[],
    }
}

/// Rotate `colors[min..=max]` right by `rotations` positions: the retail
/// colour cycle wraps the rightmost entry back to `min`.
fn rotate_palette(colors: &mut [Rgb; 256], min: usize, max: usize, rotations: u64) {
    if max >= 256 || max <= min {
        return;
    }
    let len = max - min + 1;
    let n = (rotations % len as u64) as usize;
    if n == 0 {
        return;
    }
    let mut old = [Rgb(0); 256];
    old[min..=max].copy_from_slice(&colors[min..=max]);
    for i in 0..len {
        colors[min + (i + n) % len] = old[min + i];
    }
}

impl Palette {
    pub fn from_wpe(bytes: &[u8]) -> Self {
        let mut colors = [Rgb(0); 256];
        for (i, color) in colors.iter_mut().enumerate() {
            let o = i * 4;
            if o + 3 <= bytes.len() {
                *color = Rgb::new(bytes[o], bytes[o + 1], bytes[o + 2]);
            }
        }
        Self { colors }
    }

    pub fn from_pal(bytes: &[u8]) -> Self {
        let mut colors = [Rgb(0); 256];
        for (i, color) in colors.iter_mut().enumerate() {
            let o = i * 3;
            if o + 3 <= bytes.len() {
                *color = Rgb::new(bytes[o], bytes[o + 1], bytes[o + 2]);
            }
        }
        Self { colors }
    }

    /// Retail GRPs reserve palette entries 8..=15 for the player colour.
    pub fn color(&self, index: u8, team: Option<Rgb>) -> Rgb {
        if let Some(team) = team
            && (8..=15).contains(&index)
        {
            // Palette entries 8..=15 are the player-colour ramp, brightest
            // first.  Retail fills them from `game\tunit.pcx` (8 shades per
            // player); these are the median brightness steps of its first
            // eight players, applied to the player's colour.
            const SHADE: [u32; 8] = [256, 216, 198, 162, 126, 110, 74, 42];
            let k = SHADE[(index - 8) as usize];
            let c = team.0;
            let ch = |shift: u32| ((c >> shift) & 0xff) * k / 256;
            return Rgb((ch(16) << 16) | (ch(8) << 8) | ch(0));
        }
        self.colors[index as usize]
    }
}

#[derive(Clone, Debug)]
pub struct GrpFrame {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    /// Palette indices, row-major, 0 = transparent.
    pub indices: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct Grp {
    pub width: i32,
    pub height: i32,
    pub frames: Vec<GrpFrame>,
}

fn u16_at(data: &[u8], o: usize) -> Option<u16> {
    Some(u16::from_le_bytes([*data.get(o)?, *data.get(o + 1)?]))
}

fn u32_at(data: &[u8], o: usize) -> Option<u32> {
    Some(u32::from_le_bytes([
        *data.get(o)?,
        *data.get(o + 1)?,
        *data.get(o + 2)?,
        *data.get(o + 3)?,
    ]))
}

impl Grp {
    pub fn parse(data: &[u8]) -> Option<Self> {
        let frames = u16_at(data, 0)? as usize;
        let width = u16_at(data, 2)? as i32;
        let height = u16_at(data, 4)? as i32;
        if frames == 0 || !(1..=2400).contains(&frames) || width <= 0 || height <= 0 {
            return None;
        }
        let mut out = Vec::with_capacity(frames);
        for i in 0..frames {
            let hdr = 6 + i * 8;
            let x = *data.get(hdr)? as i32;
            let y = *data.get(hdr + 1)? as i32;
            let w = *data.get(hdr + 2)? as i32;
            let h = *data.get(hdr + 3)? as i32;
            let fd = u32_at(data, hdr + 4)? as usize;
            if w <= 0 || h <= 0 || fd >= data.len() {
                out.push(GrpFrame {
                    x,
                    y,
                    w: 0,
                    h: 0,
                    indices: Vec::new(),
                });
                continue;
            }
            let mut indices = vec![0u8; (w * h) as usize];
            for row in 0..h as usize {
                let off = u16_at(data, fd + row * 2).map(|v| fd + v as usize)?;
                if off >= data.len() {
                    return None;
                }
                let mut pos = off;
                let mut col = 0usize;
                while col < w as usize {
                    let op = *data.get(pos)?;
                    pos += 1;
                    if op & 0x80 != 0 {
                        col += (op - 0x80) as usize;
                    } else if op & 0x40 != 0 {
                        let n = (op - 0x40) as usize;
                        let v = *data.get(pos)?;
                        pos += 1;
                        for _ in 0..n {
                            if col < w as usize {
                                indices[row * w as usize + col] = v;
                            }
                            col += 1;
                        }
                    } else {
                        let n = op as usize;
                        for _ in 0..n {
                            if col < w as usize {
                                indices[row * w as usize + col] = *data.get(pos)?;
                            }
                            pos += 1;
                            col += 1;
                        }
                    }
                }
            }
            out.push(GrpFrame {
                x,
                y,
                w,
                h,
                indices,
            });
        }
        Some(Self {
            width,
            height,
            frames: out,
        })
    }

    pub fn frame(&self, index: usize) -> Option<&GrpFrame> {
        if self.frames.is_empty() {
            None
        } else {
            Some(&self.frames[index.min(self.frames.len() - 1)])
        }
    }
}

/// Full tileset chain from the retail CASC storage.
pub struct Tileset {
    pub name: String,
    pub palette: Palette,
    cv5: Vec<u8>,
    vx4: Vec<u8>,
    vr4: Vec<u8>,
    /// Remastered HD megatile art (`hd2/tileset/<era>.dds.vr4`): one 32x32
    /// RGBA image per megatile, downsampled from the shipped 64x64 DXT1 DDS.
    /// When present the terrain is drawn from this instead of the SD palette
    /// composition, which is what makes the captures look like the retail HD
    /// client.
    hd: Option<Vec<Option<Box<[u32]>>>>,
    pub minimap: Vec<Rgb>,
    pub minimap_size: (usize, usize),
    /// Terrain palette before colour cycling, restored when the option is
    /// off or the animation phase is recomputed.
    base_palette: Palette,
    /// Number of palette rotations already applied.
    cycle_rotations: u64,
    /// SD creep-edge frames (`tileset/<era>.grp`, 37 palette-indexed frames
    /// with their offsets inside the 32 px tile), when the install has them.
    creep_grp: Option<Grp>,
}

pub fn era_name(era: u16) -> &'static str {
    match era {
        0 => "badlands",
        1 => "platform",
        2 => "install",
        3 => "ashworld",
        4 => "jungle",
        5 => "desert",
        6 => "ice",
        _ => "twilight",
    }
}

/// Load the Remastered HD megatile art.
///
/// `hd2/tileset/<era>.dds.vr4` is a flat archive: `[u32 total]` followed by
/// records of `[u32 flags][u16 w][u16 h][u32 len][DDS payload]`. Each payload
/// is a 64x64 DXT1 surface holding one megatile (4x4 tiles of 16 px, the HD
/// counterpart of the SD 32x32 megatile). The art is downsampled to the world
/// scale used by the renderer so the whole coordinate system stays in SD units.
pub fn load_hd_megatiles(path: &Path) -> Option<Vec<Option<Box<[u32]>>>> {
    let bytes = fs::read(path).ok()?;
    let mut out: Vec<Option<Box<[u32]>>> = Vec::new();
    let mut off = 4usize;
    while off + 16 <= bytes.len() {
        // The payload length sits right before the "DDS " magic.
        let mut found = None;
        let mut i = off;
        while i + 4 <= bytes.len() {
            if &bytes[i..i + 4] == b"DDS " {
                found = Some(i);
                break;
            }
            i += 1;
        }
        let Some(dds_at) = found else { break };
        let len = u32_at(&bytes, dds_at - 4)? as usize;
        if len < 128 || dds_at + len > bytes.len() {
            break;
        }
        let img = crate::dds::parse(&bytes[dds_at..dds_at + len]);
        out.push(img.map(|im| downsample_rgba(&im, 32, 32)));
        off = dds_at + len;
    }
    (!out.is_empty()).then_some(out)
}

/// One resampled megatile (ARGB, `size * size`).
type TilePixels = std::rc::Rc<Box<[u32]>>;

/// 4x HD terrain (`tileset/<era>.dds.vr4`, 128x128 DXT1 per megatile),
/// drawn at screen resolution.  Payloads stay compressed; a megatile is
/// decoded and resampled to the current tile size the first time it is
/// drawn (a map uses a few hundred of the ~7000).
pub struct HdTerrain {
    bytes: Vec<u8>,
    /// `(offset, len)` of each megatile's DDS payload.
    records: Vec<(usize, usize)>,
    cache: std::cell::RefCell<HashMap<(usize, usize), TilePixels>>,
    /// Megatiles whose SD pixels use a colour-cycled palette range.
    water: std::cell::RefCell<HashMap<usize, bool>>,
    /// 4x creep-edge frames (`tileset/<era>.dds.grp`) with their SD
    /// placement (`tileset/<era>.grp`: x/y offset inside the 32 px tile).
    creep_edges: Vec<(crate::dds::DdsImage, u8, u8)>,
}

/// Retail creep-edge frame of a non-creep tile from its 9-bit neighbour
/// mask (bit order (1,1) (0,1) (-1,1) (1,0) (-1,0) (1,-1) (0,-1) (-1,-1)
/// self): 0 = no edge, n = grp frame n - 1.  Construction as in OpenBW's
/// `load_image_data` (ui.h).
pub fn creep_edge_frame_table() -> [u8; 256] {
    let canon = |i: usize| {
        let mut v = 0usize;
        if i & 2 != 0 {
            v |= 0x10;
        }
        if i & 8 != 0 {
            v |= 0x24;
        }
        if i & 0x10 != 0 {
            v |= 9;
        }
        if i & 0x40 != 0 {
            v |= 2;
        }
        if i & 0xc0 == 0xc0 {
            v |= 1;
        }
        if i & 0x60 == 0x60 {
            v |= 4;
        }
        if i & 3 == 3 {
            v |= 0x20;
        }
        if i & 6 == 6 {
            v |= 8;
        }
        if v & 0x21 == 0x21 {
            v |= 0x40;
        }
        if v & 0xc == 0xc {
            v |= 0x40;
        }
        v
    };
    let canon_of: Vec<usize> = (0..256).map(canon).collect();
    let mut rank = [0u8; 128];
    let mut n = 0u8;
    for (v, r) in rank.iter_mut().enumerate() {
        if canon_of.contains(&v) {
            *r = n;
            n += 1;
        }
    }
    let mut out = [0u8; 256];
    for (i, o) in out.iter_mut().enumerate() {
        *o = rank[canon_of[i]];
    }
    out
}

impl HdTerrain {
    /// Index the archive: `[u32 file size][u16 count][u16 ?]` then
    /// `[u32 flags][u16 w][u16 h][u32 len][DDS]` records (jungle: 7038).
    pub fn parse(bytes: Vec<u8>) -> Option<Self> {
        let mut records = Vec::new();
        let mut off = 8usize;
        while off + 12 <= bytes.len() {
            let len = u32_at(&bytes, off + 8)? as usize;
            let at = off + 12;
            if len < 128 || at + len > bytes.len() || &bytes[at..at + 4] != b"DDS " {
                break;
            }
            records.push((at, len));
            off = at + len;
        }
        (!records.is_empty()).then(|| Self {
            bytes,
            records,
            cache: Default::default(),
            water: Default::default(),
            creep_edges: Vec::new(),
        })
    }

    pub fn load(root: &Path, era: u16) -> Option<Self> {
        let dir = root.join("hd/tileset");
        let name = era_name(era);
        let mut t = Self::parse(fs::read(dir.join(format!("{name}.dds.vr4"))).ok()?)?;
        let hd = fs::read(dir.join(format!("{name}.dds.grp")))
            .ok()
            .and_then(|b| crate::dds::DdsGrp::parse(&b));
        let sd = fs::read(dir.join(format!("{name}.grp"))).ok();
        if let (Some(hd), Some(sd)) = (hd, sd) {
            t.creep_edges = hd
                .frames
                .into_iter()
                .enumerate()
                .map(|(i, mut img)| {
                    // SD frame header: {u8 x, u8 y, u8 w, u8 h, u32 off}.
                    let at = 6 + i * 8;
                    let (x, y) = (
                        sd.get(at).copied().unwrap_or(0),
                        sd.get(at + 1).copied().unwrap_or(0),
                    );
                    for p in img.pixels.iter_mut() {
                        *p = apply_gain(*p);
                    }
                    (img, x, y)
                })
                .collect();
        }
        Some(t)
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Megatile `mega` resampled to `size`x`size` (premultiplied-alpha box
    /// filter over the 128 px source).
    pub fn tile(&self, mega: usize, size: usize) -> Option<TilePixels> {
        if let Some(t) = self.cache.borrow().get(&(mega, size)) {
            return Some(t.clone());
        }
        let &(at, len) = self.records.get(mega)?;
        let img = crate::dds::parse(&self.bytes[at..at + len])?;
        let mut px = downsample_rgba(&img, size, size);
        for p in px.iter_mut() {
            *p = apply_gain(*p);
        }
        let t = std::rc::Rc::new(px);
        self.cache.borrow_mut().insert((mega, size), t.clone());
        Some(t)
    }

    /// Cached "does this HD megatile overlay animated water/lava" lookup.
    fn is_water(&self, tileset: &Tileset, mega: usize) -> bool {
        if let Some(v) = self.water.borrow().get(&mega) {
            return *v;
        }
        let v = tileset.megatile_cycles(mega);
        self.water.borrow_mut().insert(mega, v);
        v
    }

    /// Draw the terrain of `scenario` into the screen rectangle `dest`
    /// (`x, y, w, h`), with the camera at game px `cam` and `s` screen px per
    /// game px.  `tileset` resolves MTXM entries to megatiles; tiles for
    /// which `creep(tx, ty)` returns an MTXM entry draw that creep tile
    /// (cv5 group 1, see [`CreepTiles`]); the others get creep edges.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &self,
        fb: &mut Framebuffer,
        tileset: &Tileset,
        scenario: &Scenario,
        cam: (i32, i32),
        dest: (i32, i32, i32, i32),
        s: f32,
        creep: &dyn Fn(i32, i32) -> Option<u16>,
        cycle: Option<u64>,
    ) {
        let size = (TILE_PX as f32 * s).round().max(1.0) as usize;
        let (dx, dy, dw, dh) = dest;
        let view_w = (dw as f32 / s).ceil() as i32;
        let view_h = (dh as f32 / s).ceil() as i32;
        let first_x = (cam.0 / TILE_PX).max(0);
        let first_y = (cam.1 / TILE_PX).max(0);
        let last_x = ((cam.0 + view_w) / TILE_PX + 1).min(scenario.width as i32);
        let last_y = ((cam.1 + view_h) / TILE_PX + 1).min(scenario.height as i32);
        // Tile list first (the megatile cache is single-threaded), then the
        // blits in parallel horizontal bands (`par_bands`), then the creep
        // edges.  Draw order inside a band is unchanged.
        let mut tiles: Vec<(i32, i32, TilePixels, bool)> = Vec::new();
        let mut edges: Vec<(i32, i32, i32, i32)> = Vec::new();
        for ty in first_y..last_y {
            for tx in first_x..last_x {
                let Some(&mtxm) = scenario
                    .tiles
                    .get(ty as usize * scenario.width + tx as usize)
                else {
                    continue;
                };
                let sx = dx + ((tx * TILE_PX - cam.0) as f32 * s).round() as i32;
                let sy = dy + ((ty * TILE_PX - cam.1) as f32 * s).round() as i32;
                let mtxm = creep(tx, ty).unwrap_or(mtxm);
                let mega = tileset.megatile(mtxm) as usize;
                if let Some(px) = self.tile(mega, size) {
                    let water = cycle.is_some() && self.is_water(tileset, mega);
                    tiles.push((sx, sy, px, water));
                }
                edges.push((tx, ty, sx, sy));
            }
        }
        let clip = (dx, dy, dx + dw, dy + dh);
        let phase = cycle.unwrap_or(0);
        let draws: Vec<(i32, i32, &[u32], Option<f32>)> = tiles
            .iter()
            .map(|(x, y, p, water)| (*x, *y, &p[..], water.then(|| water_tint(phase))))
            .collect();
        let (fw, fh) = (fb.width, fb.height);
        let stride = fw as usize;
        par_bands(&mut fb.pixels, stride, |first, band| {
            let rows = (band.len() / stride) as i32;
            let (b0, b1) = (first as i32, first as i32 + rows);
            for &(sx, sy, px, tint) in &draws {
                if sy + size as i32 <= b0 || sy >= b1 {
                    continue;
                }
                let band_clip = (clip.0, clip.1.max(b0), clip.2, clip.3.min(b1));
                blit_tile_rows(band, stride, fw, fh, b0, px, size, sx, sy, band_clip, tint);
            }
        });
        for (tx, ty, sx, sy) in edges {
            {
                if let Some(frame) = creep_edge_frame(creep, tx, ty)
                    && let Some((img, ox, oy)) = self.creep_edges.get(frame)
                {
                    let k = s / 4.0;
                    let (w, h) = (
                        (img.width as f32 * k).round() as i32,
                        (img.height as f32 * k).round() as i32,
                    );
                    // A positive SD offset means the frame hugs the
                    // tile's right / bottom edge (the HD frames are
                    // padded, so the offset itself does not scale).
                    let ex = if *ox > 0 { sx + size as i32 - w } else { sx };
                    let ey = if *oy > 0 { sy + size as i32 - h } else { sy };
                    img.draw_scaled(fb, ex, ey, w, h);
                }
            }
        }
    }
}

/// Brightness factor for HD water pixels at a colour-cycle phase.  The HD
/// megatiles are pre-coloured DDS, so the SD palette rotation cannot be
/// applied directly; a small luminance wave on the blue texels reproduces
/// the visible classic water animation at the same 8-rotations/second rate.
fn water_tint(phase: u64) -> f32 {
    let t = (phase % 16) as f32 / 16.0 * std::f32::consts::TAU;
    1.0 + 0.08 * t.sin()
}

/// Retail draws the HD terrain darker than the raw texels: lit grass and
/// dirt measure 0.91 of the DDS colour on 135051 / 135020.
fn apply_gain(p: u32) -> u32 {
    let scale = |c: u32| ((c as f32) * HD_TERRAIN_GAIN).round() as u32;
    (p & 0xff00_0000)
        | (scale((p >> 16) & 0xff) << 16)
        | (scale((p >> 8) & 0xff) << 8)
        | scale(p & 0xff)
}

/// Measured brightness of the retail HD terrain relative to its texels.
pub const HD_TERRAIN_GAIN: f32 = 0.91;

/// Minimap camera box `(x, y, w, h, stroke)` in screen px for a minimap
/// whose left edge is `x0` and width `w`, top `y0`, over a `tiles` map:
/// the view `cam` (x0, y0, x1, y1 game px) on the classic 128 px grid,
/// position floored and size ceiled, stroke one classic px.
pub fn minimap_view_box(
    (x0, w): (i32, i32),
    y0: i32,
    (mw, mh): (usize, usize),
    cam: (i32, i32, i32, i32),
) -> (i32, i32, i32, i32, i32) {
    let classic_per_tile = 128.0 / mw.max(mh).max(1) as f32;
    let q = w as f32 / (mw.max(1) as f32 * classic_per_tile);
    let to_classic = |v: i32| v as f32 / TILE_PX as f32 * classic_per_tile;
    let sx = (to_classic(cam.0).floor() * q).round() as i32;
    let sy = (to_classic(cam.1).floor() * q).round() as i32;
    let sw = (to_classic(cam.2 - cam.0).ceil() * q).round() as i32;
    let sh = (to_classic(cam.3 - cam.1).ceil() * q).round() as i32;
    (x0 + sx, y0 + sy, sw, sh, q.round().max(1.0) as i32)
}

/// Minimap background mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MinimapTerrain {
    /// Black radar: blips and camera box only.
    Hidden,
    /// One averaged colour per tile.
    Tiles,
    /// Keep what the caller already painted in the rect.
    Prepainted,
}

/// One unit on the minimap: position and placement box in game px.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MinimapDot {
    pub x: i32,
    pub y: i32,
    pub owner: u8,
    /// `units.dat` placement box (game px).
    pub placement: (i32, i32),
    pub building: bool,
    pub resource: bool,
    /// The owner's colour ([`player_rgb`]) for other players' blips.
    pub color: Rgb,
}

/// Minimap blip of a unit in whole tiles `(tx, ty, w, h)`: OpenBW's
/// `draw_minimap` rule (placement box / 32, buildings clamped to 4 tiles,
/// at least 2 x 2, top-left at `(position - placement / 2) / 32`).  FACT
/// (2026-11-03): the retail 135051 minimap matches it row for row (geyser
/// 12 x 6 px, Nexus plus probes 12 x 12, the overlapping mineral fields);
/// the earlier square as wide as the placement box did not.
pub fn minimap_blip_tiles(d: &MinimapDot) -> (i32, i32, i32, i32) {
    let min = 2;
    let (pw, ph) = d.placement;
    let (mut w, mut h) = (pw / TILE_PX, ph / TILE_PX);
    if d.building {
        w = w.min(4);
        h = h.min(4);
    }
    (
        (d.x - pw / 2).div_euclid(TILE_PX),
        (d.y - ph / 2).div_euclid(TILE_PX),
        w.max(min),
        h.max(min),
    )
}

/// Minimap colours measured on 135051: own units and resources.
pub const MINIMAP_SELF: Rgb = Rgb::new(16, 252, 24);
pub const MINIMAP_RESOURCE: Rgb = Rgb::new(0, 228, 252);

/// Fog darkness of an explored tile that is not in sight (0..=255).
pub const FOG_EXPLORED_ALPHA: f32 = 150.0;
/// Horizontal offset (game px) at which the fog grid is sampled: retail's
/// fog edges sit half a tile left of a tile-centred grid on every reference
/// (134905 / 135020 / 135051, 17-80 px at 3x).  Measured by sweep (0, 16,
/// 24, 32; best 16); vertical offsets did not help.  Unexplained, so kept as
/// a measured constant.
const FOG_SAMPLE_DX: f32 = 16.0;
/// Fog grid samples per tile, and the box-filter radius in samples.
///
/// The edge of vision is a box blur of the per-tile darkness (a linear
/// ramp 11 samples = 2.75 tiles wide).  INFERENCE (2026-11-03): retail's
/// measured ramp is close to linear (0.15 -> 0.9 over ~170 screen px on
/// 135020 / 135051), and against the previous Gaussian (sigma 0.8 tiles) the
/// box lowers the frame mean on all three references: 3.438 -> 3.428
/// (135051), 2.510 -> 2.499 (135020), 3.222 -> 3.193 (134905); widths 2.0 /
/// 3.0 tiles and two passes score worse.  Falsifier: a capture whose fog
/// edge has a visible Gaussian tail.
const FOG_SUB: i32 = 4;
const FOG_BOX_RADIUS: i32 = 5;

/// In-place separable box blur of radius `r` over a `w x h` grid (clamped
/// edges).
fn box_blur(v: &mut [f32], w: usize, h: usize, r: i32) {
    let n = (2 * r + 1) as f32;
    let mut tmp = vec![0f32; v.len()];
    for y in 0..h {
        for x in 0..w {
            let acc: f32 = (-r..=r)
                .map(|d| v[y * w + (x as i32 + d).clamp(0, w as i32 - 1) as usize])
                .sum();
            tmp[y * w + x] = acc / n;
        }
    }
    for y in 0..h {
        for x in 0..w {
            let acc: f32 = (-r..=r)
                .map(|d| tmp[(y as i32 + d).clamp(0, h as i32 - 1) as usize * w + x])
                .sum();
            v[y * w + x] = acc / n;
        }
    }
}

/// Stretch the first `src_w` columns of `rect` (`x, y, w, h`) across its
/// full width with linear filtering, sampling at pixel centres.
///
/// Retail's playfield has an integer logical width (853 game px in a
/// 2560 px 16:9 box) and is scaled to fill the box, so at 1440p one game px
/// is 2560 / 853 = 3.0012 screen px horizontally and exactly 3 vertically.
/// The HD world is drawn at 3.0 here (2559 px) and then stretched.  FACT,
/// measured on 134905 / 135051 terrain: the best horizontal shift grows
/// linearly with x (0.00 at x 120-280, 0.50 at 1240, 1.00 at 2360;
/// slope 0.00046 against 0.00039 predicted, no vertical component).
pub fn stretch_columns(fb: &mut Framebuffer, rect: (i32, i32, i32, i32), src_w: i32) {
    let (x0, y0, w, h) = rect;
    if src_w <= 0 || src_w >= w {
        return;
    }
    let k = src_w as f32 / w as f32;
    let taps: Vec<(i32, i32, f32)> = (0..w)
        .map(|x| {
            let f = ((x as f32 + 0.5) * k - 0.5).max(0.0);
            let i = (f.floor() as i32).min(src_w - 1);
            (i, (i + 1).min(src_w - 1), f - i as f32)
        })
        .collect();
    // Row slices and integer taps (weight / 256); the per-pixel
    // `get`/`put` + float version cost 14 ms of a 1080p frame.
    let taps: Vec<(usize, usize, u32)> = taps
        .iter()
        .map(|&(a, b, t)| (a as usize, b as usize, (t * 256.0).round() as u32))
        .collect();
    let (fw, fh) = (fb.width, fb.height);
    if x0 < 0 || x0 + w > fw {
        return;
    }
    let (ya, yb) = (y0.max(0) as usize, (y0 + h).min(fh).max(0) as usize);
    if ya >= yb {
        return;
    }
    let stride = fw as usize;
    let rows = &mut fb.pixels[ya * stride..yb * stride];
    par_bands(rows, stride, |_, band| {
        let mut row = vec![0u32; src_w as usize];
        for line in band.chunks_mut(stride) {
            let seg = &mut line[x0 as usize..(x0 + w) as usize];
            row.copy_from_slice(&seg[..src_w as usize]);
            for (o, &(a, b, t)) in seg.iter_mut().zip(taps.iter()) {
                let (pa, pb) = (row[a], row[b]);
                let mix = |sh: u32| {
                    let ca = (pa >> sh) & 0xff;
                    let cb = (pb >> sh) & 0xff;
                    (ca * (256 - t) + cb * t + 128) >> 8
                };
                *o = mix(16) << 16 | mix(8) << 8 | mix(0);
            }
        }
    });
}

/// Retail HD fog at screen resolution: one darkness value per tile (0 in
/// sight, [`FOG_EXPLORED_ALPHA`] explored, 255 unexplored), interpolated
/// bilinearly between tile centres so the edge of vision is a soft ramp
/// instead of 32 px blocks.
#[allow(clippy::too_many_arguments)]
pub fn draw_fog_smooth(
    fb: &mut Framebuffer,
    scenario: &Scenario,
    world: &crate::sim::World,
    owner: u8,
    cam: (i32, i32),
    dest: (i32, i32, i32, i32),
    s: f32,
) {
    let (dx, dy, dw, dh) = dest;
    // Margin of 4 tiles so the blur sees the tiles beyond the screen edge.
    let tx0 = (cam.0 / TILE_PX) - 4;
    let ty0 = (cam.1 / TILE_PX) - 4;
    let tw = (dw as f32 / s / TILE_PX as f32).ceil() as i32 + 9;
    let th = (dh as f32 / s / TILE_PX as f32).ceil() as i32 + 9;
    let (mw, mh) = (scenario.width as i32, scenario.height as i32);
    let mut grid = vec![0f32; (tw * th) as usize];
    for j in 0..th {
        for i in 0..tw {
            let (tx, ty) = ((tx0 + i).clamp(0, mw - 1), (ty0 + j).clamp(0, mh - 1));
            let (cx, cy) = (tx * TILE_PX + TILE_PX / 2, ty * TILE_PX + TILE_PX / 2);
            grid[(j * tw + i) as usize] = if !world.is_explored_by(owner, cx, cy) {
                255.0
            } else if !world.is_visible_to(owner, cx, cy) {
                FOG_EXPLORED_ALPHA
            } else {
                0.0
            };
        }
    }
    // Upsample to FOG_SUB samples per tile (piecewise constant) and box-blur
    // (see FOG_BOX_RADIUS).
    let sub = FOG_SUB;
    let (gw, gh) = (tw * sub, th * sub);
    let mut fine = vec![0f32; (gw * gh) as usize];
    for y in 0..gh {
        for x in 0..gw {
            fine[(y * gw + x) as usize] = grid[((y / sub) * tw + x / sub) as usize];
        }
    }
    box_blur(&mut fine, gw as usize, gh as usize, FOG_BOX_RADIUS);
    let step = TILE_PX as f32 / sub as f32;
    // Column taps are the same on every row: computed once (the per-pixel
    // float + clamp version was 10 ms of a 1080p frame).
    let (x_lo, x_hi) = (dx.max(0), (dx + dw).min(fb.width));
    let cols: Vec<(usize, usize, f32)> = (x_lo..x_hi)
        .map(|x| {
            let gx = cam.0 as f32 + (x - dx) as f32 / s;
            let fx = (gx + FOG_SAMPLE_DX - (tx0 * TILE_PX) as f32) / step - 0.5;
            let i = fx.floor() as i32;
            (
                i.clamp(0, gw - 1) as usize,
                (i + 1).clamp(0, gw - 1) as usize,
                fx - i as f32,
            )
        })
        .collect();
    let fbw = fb.width as usize;
    let (ya, yb) = (dy.max(0) as usize, (dy + dh).min(fb.height).max(0) as usize);
    if ya >= yb {
        return;
    }
    let fine = &fine;
    let cols = &cols;
    par_bands(&mut fb.pixels[ya * fbw..yb * fbw], fbw, |first, band| {
        for (r, row) in band.chunks_mut(fbw).enumerate() {
            let y = (ya + first + r) as i32;
            let gy = cam.1 as f32 + (y - dy) as f32 / s;
            let fy = (gy - (ty0 * TILE_PX) as f32) / step - 0.5;
            let j = fy.floor() as i32;
            let wy = fy - j as f32;
            let ra = &fine[(j.clamp(0, gh - 1) * gw) as usize..][..gw as usize];
            let rb = &fine[((j + 1).clamp(0, gh - 1) * gw) as usize..][..gw as usize];
            for (x, &(i0, i1, wx)) in (x_lo..x_hi).zip(cols.iter()) {
                let (a00, a10, a01, a11) = (ra[i0], ra[i1], rb[i0], rb[i1]);
                if a00 + a10 + a01 + a11 <= 0.0 {
                    continue;
                }
                let top = a00 + (a10 - a00) * wx;
                let bottom = a01 + (a11 - a01) * wx;
                let a = top + (bottom - top) * wy;
                if a > 0.5 {
                    let p = &mut row[x as usize];
                    *p = Rgb(*p)
                        .blend(Rgb::new(0, 0, 0), a.round().min(255.0) as u8)
                        .0;
                }
            }
        }
    });
}

/// Client-side creep tile picker: which of the 13 creep megatiles (cv5
/// group 1) each creep tile draws with.
///
/// FACT (2026-11-03, 64-bit 1.23.10 in-game dump
/// `sc_mem_ingame64_20261028_0814`, file offsets): the client picks a tile's
/// creep megatile when the tile gains creep (`0x64fb30`, reached through
/// `0x64fbe0` from the creep set/spread loops at `0x64f967` and `0x650010`):
/// `r = rand() % 100; v = rand(); tile = 0x10 + (r < 4 ? 6 + v % 7 : v % 6)`.
/// `rand` is the CRT generator at `0xa12380`
/// (`s = s * 214013 + 2531011; (s >> 16) & 0x7fff`), seeded with
/// `srand(time(NULL))` at `0x2b17e2`, so the variants are per-session state
/// and not part of the synchronised game.  Falsified on the way: OpenBW's
/// prefilled 256 x 256 table (Borland or MSVC LCG, stride 128 or 256) does
/// not reproduce the 160 tiles recorded from the 134905 capture for any of
/// the 2^32 generator states, even allowing 60 mismatches.
#[derive(Clone, Debug, Default)]
pub struct CreepTiles {
    width: usize,
    /// MTXM entry per tile (`0` = no creep).
    tiles: Vec<u16>,
}

/// The client's CRT `rand()` (64-bit 1.23.10 `0xa12380`): one stream per
/// process, shared by every non-synchronised random choice (creep tiles,
/// portrait clips, ...).  Retail seeds it with `srand(time(NULL))`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrtRand(u32);

impl CrtRand {
    pub fn new(seed: u32) -> Self {
        Self(seed)
    }

    /// `s = s * 214013 + 2531011; (s >> 16) & 0x7fff`.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(214_013).wrapping_add(2_531_011);
        (self.0 >> 16) & 0x7fff
    }
}

impl CreepTiles {
    /// Pick the MTXM entry of a tile that just gained creep (`0x64fb30`).
    pub fn pick(rand: &mut CrtRand) -> u16 {
        let roll = rand.next() % 100;
        let v = rand.next();
        0x10 | if roll < 4 { 6 + v % 7 } else { v % 6 } as u16
    }

    /// Follow the sim's creep grid (`World::creep_grid`, maturity per tile):
    /// a tile that gained creep picks its megatile, one that lost it forgets
    /// it.  A new map (different size) starts empty; the CRT stream carries
    /// on, as it does in the retail process.
    pub fn update(&mut self, rand: &mut CrtRand, grid: &[u8], width: usize) {
        if self.width != width || self.tiles.len() != grid.len() {
            self.width = width;
            self.tiles = vec![0; grid.len()];
        }
        for (tile, &maturity) in self.tiles.iter_mut().zip(grid) {
            match (maturity > 0, *tile != 0) {
                (true, false) => *tile = Self::pick(rand),
                (false, true) => *tile = 0,
                _ => {}
            }
        }
    }

    /// MTXM entry the creep tile `(tx, ty)` draws with, `None` off creep.
    pub fn get(&self, tx: i32, ty: i32) -> Option<u16> {
        if tx < 0 || ty < 0 || tx as usize >= self.width {
            return None;
        }
        let v = *self.tiles.get(ty as usize * self.width + tx as usize)?;
        (v != 0).then_some(v)
    }
}

/// Creep-edge frame (0-based index into `tileset/<era>.grp`) drawn on the
/// non-creep tile `(tx, ty)`, from its creep neighbours (OpenBW
/// `draw_tiles`); `None` on creep tiles and where no edge applies.
pub fn creep_edge_frame(
    creep: &dyn Fn(i32, i32) -> Option<u16>,
    tx: i32,
    ty: i32,
) -> Option<usize> {
    if creep(tx, ty).is_some() {
        return None;
    }
    let mut mask = 0usize;
    for (bit, (ax, ay)) in [
        (1, 1),
        (0, 1),
        (-1, 1),
        (1, 0),
        (-1, 0),
        (1, -1),
        (0, -1),
        (-1, -1),
    ]
    .into_iter()
    .enumerate()
    {
        if creep(tx + ax, ty + ay).is_some() {
            mask |= 1 << bit;
        }
    }
    (creep_edge_frame_table()[mask] as usize).checked_sub(1)
}

/// Alpha-blend a square ARGB tile clipped to `clip` (`x0, y0, x1, y1`).
/// [`blit_tile`] into a band of rows starting at frame row `band_y0`.
#[allow(clippy::too_many_arguments)]
fn blit_tile_rows(
    band: &mut [u32],
    stride: usize,
    fw: i32,
    fh: i32,
    band_y0: i32,
    px: &[u32],
    size: usize,
    sx: i32,
    sy: i32,
    clip: (i32, i32, i32, i32),
    tint: Option<f32>,
) {
    let x_lo = clip.0.max(sx).max(0);
    let x_hi = clip.2.min(sx + size as i32).min(fw);
    if x_lo >= x_hi {
        return;
    }
    for y in clip.1.max(sy).max(0)..clip.3.min(sy + size as i32).min(fh) {
        let src = &px[(y - sy) as usize * size..][..size];
        let r = (y - band_y0) as usize;
        let dst = &mut band[r * stride..(r + 1) * stride];
        for x in x_lo..x_hi {
            let p = src[(x - sx) as usize];
            let a = (p >> 24) as u8;
            if a == 0 {
                continue;
            }
            let mut fg = Rgb(p & 0x00ff_ffff);
            if let Some(factor) = tint {
                let r = (fg.0 >> 16) & 0xff;
                let g = (fg.0 >> 8) & 0xff;
                let b = fg.0 & 0xff;
                // Only the blue water/lava texels shimmer; cliffs and shore
                // art in the same megatile keep their colour.
                if b > r && b >= g {
                    let scale = |c: u32| ((c as f32 * factor).round().clamp(0.0, 255.0)) as u32;
                    fg = Rgb((scale(r) << 16) | (scale(g) << 8) | scale(b));
                }
            }
            let d = &mut dst[x as usize];
            *d = if a == 255 {
                fg.0
            } else {
                Rgb(*d).blend(fg, a).0
            };
        }
    }
}

/// Area-filter an image down to `w`x`h` (each output pixel averages the
/// exact source footprint, fractional texels weighted by coverage), in
/// premultiplied alpha; the result is straight alpha, as [`blit_tile`]
/// blends it.
///
/// At 1440p a 128 px HD megatile becomes 96 px (1.333 texels per pixel).
/// The earlier integer-boundary box averaged alternating 1- and 2-texel
/// spans and stored premultiplied colour; on the (8)The Hunters terrain the
/// area filter lowers the mean error from 1.81 / 2.55 to 1.61 / 2.50
/// (135020 / 135051).
fn downsample_rgba(img: &crate::dds::DdsImage, w: usize, h: usize) -> Box<[u32]> {
    let mut out = vec![0u32; w * h];
    if img.width == 0 || img.height == 0 || w == 0 || h == 0 {
        return out.into_boxed_slice();
    }
    let (sw, sh) = (img.width as f32 / w as f32, img.height as f32 / h as f32);
    let texel = |x: i32, y: i32| {
        let p = img.pixel(x as usize, y as usize);
        let a = (p >> 24) as f32 / 255.0;
        [
            ((p >> 16) & 0xff) as f32 * a,
            ((p >> 8) & 0xff) as f32 * a,
            (p & 0xff) as f32 * a,
            a,
        ]
    };
    // Coverage of source cells [i, i + 1) by the span [lo, hi).
    let spans = |n: usize, k: f32, max: usize| -> Vec<Vec<(i32, f32)>> {
        (0..n)
            .map(|t| {
                let (lo, hi) = (t as f32 * k, ((t + 1) as f32 * k).min(max as f32));
                (lo.floor() as i32..hi.ceil() as i32)
                    .map(|i| (i, (hi.min(i as f32 + 1.0) - lo.max(i as f32)).max(0.0)))
                    .filter(|&(_, c)| c > 0.0)
                    .collect()
            })
            .collect()
    };
    let xs = spans(w, sw, img.width);
    let ys = spans(h, sh, img.height);
    for (ty, ycov) in ys.iter().enumerate() {
        for (tx, xcov) in xs.iter().enumerate() {
            let mut acc = [0f32; 4];
            let mut area = 0.0;
            for &(sy, wy) in ycov {
                for &(sx, wx) in xcov {
                    let t = texel(sx, sy);
                    for (a, v) in acc.iter_mut().zip(t) {
                        *a += v * wx * wy;
                    }
                    area += wx * wy;
                }
            }
            if area <= 0.0 || acc[3] <= 0.0 {
                continue;
            }
            let a = acc[3] / area;
            let c = |v: f32| (v / acc[3]).round().clamp(0.0, 255.0) as u32;
            out[ty * w + tx] = (((a * 255.0).round() as u32) << 24)
                | (c(acc[0]) << 16)
                | (c(acc[1]) << 8)
                | c(acc[2]);
        }
    }
    out.into_boxed_slice()
}

impl Tileset {
    pub fn load(dir: &Path, era: u16, scenario: &Scenario) -> Option<Self> {
        let name = era_name(era);
        let load = |ext: &str| fs::read(dir.join(format!("{name}.{ext}"))).ok();
        let cv5 = load("cv5")?;
        let vx4 = load("vx4ex")?;
        let vr4 = load("vr4")?;
        let palette = Palette::from_wpe(&load("wpe")?);
        let mut ts = Self {
            name: name.to_string(),
            base_palette: palette.clone(),
            palette,
            cycle_rotations: 0,
            cv5,
            vx4,
            vr4,
            minimap: Vec::new(),
            minimap_size: (0, 0),
            creep_grp: dir
                .parent()
                .and_then(|root| fs::read(root.join("hd/tileset").join(format!("{name}.grp"))).ok())
                .and_then(|b| Grp::parse(&b)),
            hd: dir
                .parent()
                .and_then(|root| {
                    root.join("hd2/tileset")
                        .join(format!("{name}.dds.vr4"))
                        .to_str()
                        .map(|p| load_hd_megatiles(Path::new(p)))
                })
                .flatten(),
        };
        ts.build_minimap(scenario);
        Some(ts)
    }

    /// Apply the retail terrain colour cycles (water, lava and the
    /// `248..254` glow range) at `game_ticks`.  One rotation happens every
    /// eight classic 15.615 ms ticks (125 ms), i.e. every three simulation
    /// ticks at 24 Hz.  Disabling the option restores the base palette.
    pub fn update_color_cycle(&mut self, game_ticks: u64, enabled: bool) {
        if !enabled {
            if self.cycle_rotations != 0 {
                self.palette = self.base_palette.clone();
                self.cycle_rotations = 0;
            }
            return;
        }
        let rotations = game_ticks / 3;
        if rotations == self.cycle_rotations {
            return;
        }
        self.cycle_rotations = rotations;
        self.palette = self.base_palette.clone();
        for &(min, max) in color_cycle_ranges(&self.name) {
            rotate_palette(&mut self.palette.colors, min, max, rotations);
        }
    }

    /// Whether a megatile's SD art uses one of this tileset's colour-cycled
    /// palette ranges: the HD terrain uses this to animate water/lava even
    /// though its megatiles are pre-coloured DDS surfaces.
    pub fn megatile_cycles(&self, mega: usize) -> bool {
        let ranges = color_cycle_ranges(&self.name);
        if ranges.is_empty() {
            return false;
        }
        for k in 0..16 {
            let Some(vid) = u32_at(&self.vx4, mega * 64 + k * 4) else {
                continue;
            };
            let vr = (vid >> 1) as usize;
            let Some(slice) = self.vr4.get(vr * 64..vr * 64 + 64) else {
                continue;
            };
            if slice.iter().any(|c| {
                ranges
                    .iter()
                    .any(|&(min, max)| (*c as usize) >= min && (*c as usize) <= max)
            }) {
                return true;
            }
        }
        false
    }

    /// The tileset's `cv5` group table (MTXM entry -> megatile).
    pub fn cv5(&self) -> &[u8] {
        &self.cv5
    }

    #[inline]
    pub fn megatile(&self, mtxm: u16) -> u16 {
        let group = ((mtxm >> 4) & 0x7ff) as usize;
        let index = (mtxm & 0x0f) as usize;
        u16_at(&self.cv5, group * 52 + 20 + index * 2).unwrap_or(0)
    }

    /// Paint one megatile (4x4 minitiles, 32x32 px) at `(sx, sy)`.
    fn draw_megatile(&self, fb: &mut Framebuffer, mega: usize, sx: i32, sy: i32) {
        let base = mega * 64;
        for my in 0..4 {
            for mx in 0..4 {
                let Some(vid) = u32_at(&self.vx4, base + (my * 4 + mx) * 4) else {
                    continue;
                };
                let flip = vid & 1 != 0;
                let vr = (vid >> 1) as usize;
                let vbase = vr * 64;
                for py in 0..8 {
                    for px in 0..8 {
                        let src = vbase + py * 8 + if flip { 7 - px } else { px };
                        let Some(&c) = self.vr4.get(src) else {
                            continue;
                        };
                        fb.put(
                            sx + mx as i32 * 8 + px as i32,
                            sy + my as i32 * 8 + py as i32,
                            self.palette.colors[c as usize],
                        );
                    }
                }
            }
        }
    }

    /// Fill one 32x32 map tile at `(sx, sy)`.
    ///
    /// Two layers, like the retail engine: the group background first, then
    /// the tile's own megatile (whose transparent pixels reveal the
    /// background).
    pub fn draw_tile(&self, fb: &mut Framebuffer, mtxm: u16, sx: i32, sy: i32) {
        let mega = self.megatile(mtxm) as usize;
        if self.hd.is_some() {
            self.draw_hd_megatile(fb, mega, sx, sy);
        } else {
            self.draw_megatile(fb, mega, sx, sy);
        }
    }

    /// Paint one megatile from the HD art (32x32 px at world scale), honouring
    /// the DXT1 cut-out alpha the retail terrain uses for cliff edges.
    fn draw_hd_megatile(&self, fb: &mut Framebuffer, mega: usize, sx: i32, sy: i32) {
        let Some(hd) = &self.hd else {
            return;
        };
        let Some(Some(px)) = hd.get(mega) else {
            // Missing in the HD set: fall back to the SD composition.
            self.draw_megatile(fb, mega, sx, sy);
            return;
        };
        for y in 0..32 {
            for x in 0..32 {
                let p = px[y * 32 + x];
                let a = (p >> 24) as u8;
                if a == 0 {
                    continue;
                }
                let fg = Rgb(p & 0x00ff_ffff);
                let (px_x, px_y) = (sx + x as i32, sy + y as i32);
                if a == 255 {
                    fb.put(px_x, px_y, fg);
                } else {
                    let bg = fb.get(px_x, px_y);
                    fb.put(px_x, px_y, bg.blend(fg, a));
                }
            }
        }
    }

    pub fn draw_terrain(&self, fb: &mut Framebuffer, scenario: &Scenario, cam: (i32, i32)) {
        if scenario.width == 0 || scenario.height == 0 {
            return;
        }
        let first_x = (cam.0 / TILE_PX).max(0);
        let first_y = (cam.1 / TILE_PX).max(0);
        let last_x = ((cam.0 + fb.width) / TILE_PX + 1).min(scenario.width as i32);
        let last_y = ((cam.1 + fb.height) / TILE_PX + 1).min(scenario.height as i32);
        for ty in first_y..last_y {
            for tx in first_x..last_x {
                let idx = ty as usize * scenario.width + tx as usize;
                let Some(&mtxm) = scenario.tiles.get(idx) else {
                    continue;
                };
                self.draw_tile(fb, mtxm, tx * TILE_PX - cam.0, ty * TILE_PX - cam.1);
            }
        }
    }

    /// Draw creep over the terrain as the retail client does (OpenBW
    /// `draw_tiles`): every tile for which `creep` returns an MTXM entry
    /// (cv5 group 1, see [`CreepTiles`]) draws that tile, and the other
    /// tiles draw their creep-edge frame from `tileset/<era>.grp` at the
    /// frame's offset.  The art is static: retail does not animate creep.
    pub fn draw_creep(
        &self,
        fb: &mut Framebuffer,
        scenario: &Scenario,
        cam: (i32, i32),
        creep: &dyn Fn(i32, i32) -> Option<u16>,
    ) {
        let first_x = (cam.0 / TILE_PX).max(0);
        let first_y = (cam.1 / TILE_PX).max(0);
        let last_x = ((cam.0 + fb.width) / TILE_PX + 1).min(scenario.width as i32);
        let last_y = ((cam.1 + fb.height) / TILE_PX + 1).min(scenario.height as i32);
        for ty in first_y..last_y {
            for tx in first_x..last_x {
                let (sx, sy) = (tx * TILE_PX - cam.0, ty * TILE_PX - cam.1);
                if let Some(mtxm) = creep(tx, ty) {
                    self.draw_tile(fb, mtxm, sx, sy);
                    continue;
                }
                let Some(frame) = creep_edge_frame(creep, tx, ty)
                    .and_then(|f| self.creep_grp.as_ref()?.frames.get(f))
                else {
                    continue;
                };
                for y in 0..frame.h {
                    for x in 0..frame.w {
                        let c = frame.indices[(y * frame.w + x) as usize];
                        if c != 0 {
                            fb.put(
                                sx + frame.x + x,
                                sy + frame.y + y,
                                self.palette.colors[c as usize],
                            );
                        }
                    }
                }
            }
        }
    }

    /// Draw the viewport with rules-level fog: unexplored tiles are black and
    /// explored-but-not-visible tiles are dimmed.
    pub fn draw_terrain_fog(
        &self,
        fb: &mut Framebuffer,
        scenario: &Scenario,
        cam: (i32, i32),
        world: &crate::sim::World,
        owner: u8,
    ) {
        self.draw_terrain(fb, scenario, cam);
        self.draw_fog(fb, scenario, cam, world, owner);
    }

    /// Apply the rules-level fog mask to whatever is already in the
    /// framebuffer.  Split out from `draw_terrain_fog` so the client can draw
    /// the creep overlay between the terrain and the fog.
    pub fn draw_fog(
        &self,
        fb: &mut Framebuffer,
        scenario: &Scenario,
        cam: (i32, i32),
        world: &crate::sim::World,
        owner: u8,
    ) {
        let first_x = (cam.0 / TILE_PX).max(0);
        let first_y = (cam.1 / TILE_PX).max(0);
        let last_x = ((cam.0 + fb.width) / TILE_PX + 1).min(scenario.width as i32);
        let last_y = ((cam.1 + fb.height) / TILE_PX + 1).min(scenario.height as i32);
        for ty in first_y..last_y {
            for tx in first_x..last_x {
                let px = tx * TILE_PX - cam.0;
                let py = ty * TILE_PX - cam.1;
                let cx = tx * TILE_PX + TILE_PX / 2;
                let cy = ty * TILE_PX + TILE_PX / 2;
                if !world.is_explored_by(owner, cx, cy) {
                    fb.rect(px, py, TILE_PX, TILE_PX, Rgb::new(0, 0, 0));
                } else if !world.is_visible_to(owner, cx, cy) {
                    for yy in 0..TILE_PX {
                        for xx in 0..TILE_PX {
                            let c = fb.get(px + xx, py + yy);
                            fb.put(px + xx, py + yy, c.blend(Rgb::new(0, 0, 0), 150));
                        }
                    }
                }
            }
        }
    }

    fn build_minimap(&mut self, scenario: &Scenario) {
        let (w, h) = (scenario.width, scenario.height);
        self.minimap_size = (w, h);
        self.minimap = vec![Rgb::new(10, 10, 14); w * h];
        if w == 0 || h == 0 {
            return;
        }
        for ty in 0..h {
            for tx in 0..w {
                let mtxm = scenario.tiles[ty * w + tx];
                let mega = self.megatile(mtxm) as usize;
                let base = mega * 64;
                let mut sum = (0u32, 0u32, 0u32);
                let mut n = 0u32;
                for k in 0..16 {
                    let vid = u32_at(&self.vx4, base + k * 4).unwrap_or(0);
                    let vr = (vid >> 1) as usize;
                    let vbase = vr * 64;
                    // Sample four pixels from the minitile.
                    for p in [10usize, 26, 37, 53] {
                        let c = *self.vr4.get(vbase + p).unwrap_or(&0);
                        if c != 0 {
                            let rgb = self.palette.colors[c as usize].0;
                            sum.0 += (rgb >> 16) & 0xff;
                            sum.1 += (rgb >> 8) & 0xff;
                            sum.2 += rgb & 0xff;
                            n += 1;
                        }
                    }
                }
                let n = n.max(1);
                self.minimap[ty * w + tx] =
                    Rgb::new((sum.0 / n) as u8, (sum.1 / n) as u8, (sum.2 / n) as u8);
            }
        }
    }

    pub fn draw_minimap(
        &self,
        fb: &mut Framebuffer,
        rect: (i32, i32, i32, i32),
        units: &[MinimapDot],
        cam: (i32, i32, i32, i32),
    ) {
        self.draw_minimap_with_terrain(fb, rect, units, cam, None, 0, MinimapTerrain::Tiles)
    }

    /// Radar with retail fog of war: unexplored tiles are black and explored
    /// but not currently visible ones are dimmed.  `MinimapTerrain::Hidden`
    /// is the black radar most references show (blips and camera box only);
    /// `Prepainted` keeps a background the caller already drew (the HD
    /// terrain minimap, `sc_view::draw_hud_minimap`).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_minimap_with_terrain(
        &self,
        fb: &mut Framebuffer,
        rect: (i32, i32, i32, i32),
        units: &[MinimapDot],
        cam: (i32, i32, i32, i32),
        fog: Option<&crate::vision::Vision>,
        owner: u8,
        terrain: MinimapTerrain,
    ) {
        let (x, y, w, h) = rect;
        let (mw, mh) = self.minimap_size;
        if mw == 0 || mh == 0 {
            return;
        }
        let sx = w as f32 / mw as f32;
        let sy = h as f32 / mh as f32;
        if terrain == MinimapTerrain::Hidden {
            fb.rect(x, y, w, h, Rgb::new(0, 0, 0));
        }
        for ty in 0..mh {
            if terrain != MinimapTerrain::Tiles {
                break;
            }
            for tx in 0..mw {
                let mut c = self.minimap[ty * mw + tx];
                if let Some(v) = fog {
                    let (wx, wy) = (tx as i32 * TILE_PX, ty as i32 * TILE_PX);
                    if !v.is_explored(owner, wx, wy) {
                        c = Rgb::new(0, 0, 0);
                    } else if !v.is_visible(owner, wx, wy) {
                        c = c.blend(Rgb::new(0, 0, 0), 140);
                    }
                }
                let x0 = x + (tx as f32 * sx) as i32;
                let y0 = y + (ty as f32 * sy) as i32;
                let x1 = x + ((tx + 1) as f32 * sx) as i32;
                let y1 = y + ((ty + 1) as f32 * sy) as i32;
                fb.rect(x0, y0, (x1 - x0).max(1), (y1 - y0).max(1), c);
            }
        }
        // Retail blips (`minimap_blip_tiles`): local units green, resources
        // cyan, others in player colour, each with a 1 px black outline
        // drawn under every fill, so overlapping blips share one outline
        // around their union (134905, visible over the terrain minimap).
        let rects: Vec<(i32, i32, i32, i32, Rgb)> = units
            .iter()
            .map(|d| {
                let (tx, ty, tw, th) = minimap_blip_tiles(d);
                let px = x + (tx as f32 * sx) as i32;
                let py = y + (ty as f32 * sy) as i32;
                let bw = x + ((tx + tw) as f32 * sx) as i32 - px;
                let bh = y + ((ty + th) as f32 * sy) as i32 - py;
                let c = if d.resource {
                    MINIMAP_RESOURCE
                } else if d.owner == owner {
                    MINIMAP_SELF
                } else {
                    d.color
                };
                (px, py, bw, bh, c)
            })
            .collect();
        for &(px, py, bw, bh, _) in &rects {
            fb.rect(px - 1, py - 1, bw + 2, bh + 2, Rgb::new(0, 0, 0));
        }
        for &(px, py, bw, bh, c) in &rects {
            fb.rect(px, py, bw, bh, c);
        }
        // Camera viewport, on the classic 128 px minimap grid (134905 /
        // 135051: every edge reproduced): position floored and size ceiled
        // in classic px, scaled to the screen, with a 1-classic-px white
        // stroke outside the view.
        let (vx, vy, vw, vh, t) = minimap_view_box((x, w), y, (mw, mh), cam);
        let white = Rgb::new(255, 255, 255);
        fb.rect(vx - t, vy - t, vw + 2 * t, t, white);
        fb.rect(vx - t, vy + vh, vw + 2 * t, t, white);
        fb.rect(vx - t, vy, t, vh, white);
        fb.rect(vx + vw, vy, t, vh, white);
    }
}

fn sprite_path(type_id: u16) -> Option<&'static str> {
    Some(match type_id {
        uid::MARINE => "terran/marine.grp",
        uid::GHOST => "terran/ghost.grp",
        uid::VULTURE => "terran/vulture.grp",
        uid::GOLIATH => "terran/goliath.grp",
        uid::SIEGE_TANK | uid::SIEGE_MODE => "terran/tank.grp",
        uid::SCV => "terran/scv.grp",
        uid::SCIENCE_VESSEL => "terran/wessel.grp",
        uid::DROPSHIP => "terran/dropship.grp",
        uid::BATTLECRUISER => "terran/battlecr.grp",
        uid::SPIDER_MINE => "terran/spider.grp",
        uid::FIREBAT => "terran/firebat.grp",
        uid::MEDIC => "terran/medic.grp",
        uid::ZERGLING => "zerg/zergling.grp",
        uid::HYDRALISK => "zerg/hydra.grp",
        uid::ULTRALISK => "zerg/ultra.grp",
        uid::BROODLING => "zerg/brood.grp",
        uid::DRONE => "zerg/drone.grp",
        uid::OVERLORD => "zerg/overlord.grp",
        uid::MUTALISK => "zerg/mutalid.grp",
        uid::GUARDIAN => "zerg/guardian.grp",
        uid::QUEEN => "zerg/queen.grp",
        uid::DEFILER => "zerg/defiler.grp",
        uid::COCOON => "zerg/cocoon.grp",
        uid::EGG => "zerg/egg.grp",
        uid::LURKER_EGG => "zerg/lurkegg.grp",
        uid::LARVA => "zerg/larva.grp",
        uid::LURKER => "zerg/lurker.grp",
        0xcd | 0xce => "unit/thingy/tileset/install/crdoor1.grp",
        0xcf | 0xd0 => "unit/thingy/tileset/install/didoor1.grp",
        uid::CORSAIR => "protoss/corsair.grp",
        uid::DARK_TEMPLAR => "protoss/dtemplar.grp",
        uid::DARK_ARCHON => "protoss/darchn.grp",
        uid::PROBE => "protoss/probe.grp",
        uid::ZEALOT => "protoss/zealot.grp",
        uid::DRAGOON => "protoss/dragoon.grp",
        uid::HIGH_TEMPLAR => "protoss/templar.grp",
        uid::ARCHON => "protoss/archon.grp",
        uid::SHUTTLE => "protoss/shuttle.grp",
        uid::SCOUT => "protoss/scout.grp",
        uid::ARBITER => "protoss/arbiter.grp",
        uid::CARRIER => "protoss/carrier.grp",
        uid::INTERCEPTOR => "protoss/intercep.grp",
        uid::REAVER => "protoss/trilob.grp",
        uid::OBSERVER => "protoss/witness.grp",
        uid::SUPPLY_DEPOT => "terran/depot.grp",
        uid::REFINERY => "terran/refinery.grp",
        uid::BARRACKS => "terran/tbarrack.grp",
        uid::ACADEMY => "terran/academy.grp",
        uid::FACTORY => "terran/factory.grp",
        uid::STARPORT => "terran/starport.grp",
        uid::MISSILE_TURRET => "terran/missile.grp",
        uid::BUNKER => "terran/pillbox.grp",
        uid::COMSAT_STATION => "terran/comsat.grp",
        uid::NUCLEAR_SILO => "terran/nukesilo.grp",
        uid::CONTROL_TOWER => "terran/control.grp",
        uid::PHYSICS_LAB => "terran/physics.grp",
        uid::MACHINE_SHOP => "terran/machines.grp",
        uid::COMMAND_CENTER => "neutral/starbase.grp",
        uid::NEXUS => "protoss/nexus.grp",
        uid::GATEWAY => "protoss/gateway.grp",
        uid::ROBOTICS_FACILITY => "protoss/robotic.grp",
        uid::STARGATE => "protoss/stargate.grp",
        uid::PYLON => "protoss/pylon.grp",
        uid::ASSIMILATOR => "protoss/assim.grp",
        uid::FORGE => "protoss/forge.grp",
        uid::CYBERNETICS_CORE => "protoss/gencore.grp",
        uid::CITADEL_OF_ADUN => "protoss/citadel.grp",
        uid::TEMPLAR_ARCHIVES => "protoss/archives.grp",
        uid::PHOTON_CANNON => "protoss/photon.grp",
        uid::SHIELD_BATTERY => "protoss/sbattery.grp",
        uid::HATCHERY => "zerg/hatchery.grp",
        uid::LAIR => "zerg/lair.grp",
        uid::HIVE => "zerg/hive.grp",
        uid::EXTRACTOR => "zerg/extract.grp",
        uid::SPIRE | uid::GREATER_SPIRE => "zerg/spire.grp",
        uid::QUEENS_NEST => "zerg/nest.grp",
        uid::NYDUS_CANAL => "zerg/nyduspit.grp",
        uid::SUNKEN_COLONY => "zerg/scolony.grp",
        uid::CREEP_COLONY => "zerg/fcolony.grp",
        uid::MINERAL_FIELD => "neutral/min01.grp",
        uid::MINERAL_FIELD_2 => "neutral/min02.grp",
        uid::MINERAL_FIELD_3 => "neutral/min03.grp",
        uid::VESPENE_GEYSER => "neutral/geyser.grp",
        _ => return None,
    })
}

/// Draw one classic GRP frame scaled into a destination rectangle.  Used by
/// the retail console (command buttons, wireframes) and score screen.
/// Frame of an SD unit GRP.  Mobile GRPs are laid out as pose * 17 +
/// direction; a known iscript pose (`playfram` operand, already a multiple
/// of 17) wins, else the walk cycle is guessed from the tick while moving.
/// Buildings, resources and other static GRPs use their first frame.
/// Direction frame and mirror flag of a facing byte.  A directional GRP
/// stores 17 frames per pose for the 32 retail directions: frames 0..=16
/// are north clockwise to south, and the western half is the eastern one
/// drawn mirrored (direction `d > 16` shows frame `32 - d` flipped).
pub fn sd_direction(facing: u8) -> (usize, bool) {
    let dir = ((facing as i32 + 4) / 8).rem_euclid(32) as usize;
    if dir <= 16 {
        (dir, false)
    } else {
        (32 - dir, true)
    }
}

/// Whether a GRP of `frames` frames is drawn mirrored for `facing`.
pub fn sd_frame_flip(frames: usize, facing: u8) -> bool {
    frames >= 17 && sd_direction(facing).1
}

pub fn sd_frame_index(
    frames: usize,
    facing: u8,
    pose: Option<u16>,
    moving: bool,
    tick: u64,
) -> usize {
    if frames < 17 {
        return 0;
    }
    let dir = sd_direction(facing).0;
    if let Some(raw) = pose
        && (raw as usize) + dir < frames
    {
        return raw as usize + dir;
    }
    let per_dir = (frames / 17).max(1);
    let anim = if moving {
        (1 + (tick / 5) as usize) % per_dir
    } else {
        0
    };
    (anim * 17 + dir).min(frames - 1)
}

/// Draw GRP frame `index` centred on `screen` at 1x (bullets, explosions,
/// fire in the SD view), with transparent index 0.
pub fn draw_grp_centered(
    fb: &mut Framebuffer,
    grp: &Grp,
    index: usize,
    palette: &Palette,
    screen: (i32, i32),
) {
    draw_grp_posed(fb, grp, index, palette, screen, false, GrpInk::Solid(None));
}

/// A retail remap table (`tileset\<era>\ofire.pcx`, `gfire`, `bfire`,
/// `bexpl`): 256 columns, one row per glow strength.  A glow image
/// (`images.dat` draw function 9) does not carry colours: each pixel value
/// `p` replaces the screen's palette index `d` with `table[p - 1][d]`.
pub struct RemapTable {
    rows: usize,
    data: Vec<u8>,
    colors: [Rgb; 256],
    /// Screen colour -> palette index, filled as colours are met.
    nearest: std::cell::RefCell<HashMap<u32, u8>>,
}

impl RemapTable {
    /// Decode an 8-bit, single-plane PCX with its trailing palette.
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < 128 + 769 || bytes[0] != 10 || bytes[3] != 8 || bytes[65] != 1 {
            return None;
        }
        let u16_at = |o: usize| u16::from_le_bytes([bytes[o], bytes[o + 1]]) as usize;
        let width = u16_at(8) - u16_at(4) + 1;
        let rows = u16_at(10) - u16_at(6) + 1;
        let stride = u16_at(66);
        if width != 256 || stride < width || rows == 0 {
            return None;
        }
        let pal = bytes.len() - 768;
        if bytes[pal - 1] != 12 {
            return None;
        }
        let mut raw = Vec::with_capacity(stride * rows);
        let mut p = 128;
        while raw.len() < stride * rows && p < pal - 1 {
            let b = bytes[p];
            p += 1;
            if b >= 0xc0 {
                let v = *bytes.get(p)?;
                p += 1;
                raw.extend(std::iter::repeat_n(v, (b & 0x3f) as usize));
            } else {
                raw.push(b);
            }
        }
        if raw.len() < stride * rows {
            return None;
        }
        let mut data = Vec::with_capacity(256 * rows);
        for r in 0..rows {
            data.extend_from_slice(&raw[r * stride..r * stride + 256]);
        }
        let mut colors = [Rgb(0); 256];
        for (i, c) in colors.iter_mut().enumerate() {
            *c = Rgb::new(
                bytes[pal + 3 * i],
                bytes[pal + 3 * i + 1],
                bytes[pal + 3 * i + 2],
            );
        }
        Some(Self {
            rows,
            data,
            colors,
            nearest: std::cell::RefCell::new(HashMap::new()),
        })
    }

    /// Palette index closest to a screen colour.
    fn index_of(&self, color: Rgb) -> u8 {
        if let Some(i) = self.nearest.borrow().get(&color.0) {
            return *i;
        }
        let ch = |c: u32, s: u32| ((c >> s) & 0xff) as i32;
        let best = (0..256usize)
            .min_by_key(|&i| {
                let p = self.colors[i].0;
                (0..3)
                    .map(|k| {
                        let d = ch(p, 8 * k) - ch(color.0, 8 * k);
                        d * d
                    })
                    .sum::<i32>()
            })
            .unwrap_or(0) as u8;
        self.nearest.borrow_mut().insert(color.0, best);
        best
    }

    /// What a glow pixel of strength `p` (1-based) turns `under` into.
    pub fn apply(&self, p: u8, under: Rgb) -> Rgb {
        if p == 0 {
            return under;
        }
        let row = (p as usize - 1).min(self.rows - 1);
        self.colors[self.data[row * 256 + self.index_of(under) as usize] as usize]
    }
}

/// Draw GRP frame `index` centred on `screen` through a remap table (the
/// retail way to draw fire, heal, lockdown and engine glows).
pub fn draw_grp_remapped(
    fb: &mut Framebuffer,
    grp: &Grp,
    index: usize,
    screen: (i32, i32),
    flip: bool,
    table: &RemapTable,
) {
    let Some(frame) = grp.frame(index) else {
        return;
    };
    let fx = if flip {
        grp.width - frame.x - frame.w
    } else {
        frame.x
    };
    let ox = screen.0 - grp.width / 2 + fx;
    let oy = screen.1 - grp.height / 2 + frame.y;
    for y in 0..frame.h {
        for x in 0..frame.w {
            let sx = if flip { frame.w - 1 - x } else { x };
            let c = frame.indices[(y * frame.w + sx) as usize];
            if c != 0 && ox + x >= 0 && oy + y >= 0 && ox + x < fb.width && oy + y < fb.height {
                let under = fb.get(ox + x, oy + y);
                fb.put(ox + x, oy + y, table.apply(c, under));
            }
        }
    }
}

/// How [`draw_grp_posed`] writes a pixel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GrpInk {
    /// Palette colour, with the team colour ramp when given.
    Solid(Option<Rgb>),
    /// Light added to what is already there (engine glows; `images.dat`
    /// draw function 9).  INFERENCE: retail remaps through `ofire.pcx` on
    /// the paletted screen; this true-colour view adds a warm tint scaled
    /// by the pixel value instead.
    Glow(Rgb),
}

/// Draw GRP frame `index` centred on `screen`, optionally mirrored (the
/// western directions of a directional GRP).
pub fn draw_grp_posed(
    fb: &mut Framebuffer,
    grp: &Grp,
    index: usize,
    palette: &Palette,
    screen: (i32, i32),
    flip: bool,
    ink: GrpInk,
) {
    let Some(frame) = grp.frame(index) else {
        return;
    };
    let fx = if flip {
        grp.width - frame.x - frame.w
    } else {
        frame.x
    };
    let ox = screen.0 - grp.width / 2 + fx;
    let oy = screen.1 - grp.height / 2 + frame.y;
    for y in 0..frame.h {
        for x in 0..frame.w {
            let sx = if flip { frame.w - 1 - x } else { x };
            let c = frame.indices[(y * frame.w + sx) as usize];
            if c == 0 {
                continue;
            }
            let out = match ink {
                GrpInk::Solid(team) => palette.color(c, team),
                GrpInk::Glow(tint) => {
                    let d = fb.get(ox + x, oy + y).0;
                    // The pixel value is the glow's strength (`tscGlow`:
                    // 1 at the rim up to 46 on the two exhausts).
                    // Weak pixels only lighten what is under them; strong
                    // ones take the tint (the remap tables go from a faint
                    // haze to the saturated colour).
                    let k = (c as u32).min(48);
                    let add = |shift: u32| {
                        let t = (tint.0 >> shift) & 0xff;
                        let colour = (255 * (48 - k) + t * k) / 48;
                        (((d >> shift) & 0xff) + colour * k / 64).min(255) << shift
                    };
                    Rgb(add(16) | add(8) | add(0))
                }
            };
            fb.put(ox + x, oy + y, out);
        }
    }
}

pub fn draw_grp_frame(
    fb: &mut Framebuffer,
    grp: &Grp,
    index: usize,
    palette: &Palette,
    dest: (i32, i32, i32, i32),
    team: Option<Rgb>,
) {
    if std::env::var("GC_DEBUG_GRP").is_ok() {
        eprintln!("draw_grp_frame {index} dest {dest:?} team {team:?}");
    }
    let (dx, dy, dw, dh) = dest;
    if dw <= 0 || dh <= 0 {
        return;
    }
    let Some(frame) = grp.frame(index) else {
        return;
    };
    if frame.w <= 0 || frame.h <= 0 {
        return;
    }
    for y in 0..dh {
        let sy = (y as i64 * frame.h as i64 / dh as i64) as i32;
        for x in 0..dw {
            let sx = (x as i64 * frame.w as i64 / dw as i64) as i32;
            let c = frame.indices[(sy * frame.w + sx) as usize];
            if c != 0 {
                fb.put(dx + x, dy + y, palette.color(c, team));
            }
        }
    }
}

/// Draw a GRP frame as a flat single-colour silhouette.
///
/// Retail command icons are one-colour glyphs; tinting every non-transparent
/// pixel reproduces that regardless of which palette indices the GRP uses.
pub fn draw_grp_silhouette(
    fb: &mut Framebuffer,
    grp: &Grp,
    index: usize,
    dest: (i32, i32, i32, i32),
    color: Rgb,
) {
    let (dx, dy, dw, dh) = dest;
    if dw <= 0 || dh <= 0 {
        return;
    }
    let Some(frame) = grp.frame(index) else {
        return;
    };
    if frame.w <= 0 || frame.h <= 0 {
        return;
    }
    for y in 0..dh {
        let sy = (y as i64 * frame.h as i64 / dh as i64) as i32;
        for x in 0..dw {
            let sx = (x as i64 * frame.w as i64 / dw as i64) as i32;
            if frame.indices[(sy * frame.w + sx) as usize] != 0 {
                fb.put(dx + x, dy + y, color);
            }
        }
    }
}

pub struct SpriteBank {
    grps: HashMap<u16, Grp>,
    palette: Palette,
}
impl SpriteBank {
    /// Whether the bank has a sprite sheet for this unit type.
    pub fn has_unit(&self, type_id: u16) -> bool {
        self.grps.contains_key(&type_id)
    }

    pub fn load(asset_root: &Path) -> Self {
        let pal = fs::read(asset_root.join("unit.pal"))
            .or_else(|_| fs::read(asset_root.join("units.pal")))
            .ok()
            .map(|b| Palette::from_pal(&b))
            .unwrap_or_else(|| Palette {
                colors: [Rgb::new(180, 180, 180); 256],
            });
        let mut grps = HashMap::new();
        for &id in &[
            uid::MARINE,
            uid::GHOST,
            uid::VULTURE,
            uid::GOLIATH,
            uid::SIEGE_TANK,
            uid::SIEGE_MODE,
            uid::SCV,
            uid::SCIENCE_VESSEL,
            uid::DROPSHIP,
            uid::BATTLECRUISER,
            uid::SPIDER_MINE,
            uid::FIREBAT,
            uid::MEDIC,
            uid::ZERGLING,
            uid::HYDRALISK,
            uid::ULTRALISK,
            uid::BROODLING,
            uid::DRONE,
            uid::OVERLORD,
            uid::MUTALISK,
            uid::GUARDIAN,
            uid::QUEEN,
            uid::DEFILER,
            uid::COCOON,
            uid::EGG,
            uid::LURKER_EGG,
            uid::LARVA,
            uid::LURKER,
            0xcd,
            0xce,
            0xcf,
            0xd0,
            uid::CORSAIR,
            uid::DARK_TEMPLAR,
            uid::DARK_ARCHON,
            uid::PROBE,
            uid::ZEALOT,
            uid::DRAGOON,
            uid::HIGH_TEMPLAR,
            uid::ARCHON,
            uid::SHUTTLE,
            uid::SCOUT,
            uid::ARBITER,
            uid::CARRIER,
            uid::INTERCEPTOR,
            uid::REAVER,
            uid::OBSERVER,
            uid::MINERAL_FIELD,
            uid::MINERAL_FIELD_2,
            uid::MINERAL_FIELD_3,
            uid::VESPENE_GEYSER,
            uid::SUPPLY_DEPOT,
            uid::REFINERY,
            uid::BARRACKS,
            uid::ACADEMY,
            uid::FACTORY,
            uid::STARPORT,
            uid::MISSILE_TURRET,
            uid::BUNKER,
            uid::COMSAT_STATION,
            uid::NUCLEAR_SILO,
            uid::CONTROL_TOWER,
            uid::PHYSICS_LAB,
            uid::MACHINE_SHOP,
            uid::COMMAND_CENTER,
            uid::NEXUS,
            uid::GATEWAY,
            uid::ROBOTICS_FACILITY,
            uid::STARGATE,
            uid::PYLON,
            uid::ASSIMILATOR,
            uid::FORGE,
            uid::CYBERNETICS_CORE,
            uid::CITADEL_OF_ADUN,
            uid::TEMPLAR_ARCHIVES,
            uid::PHOTON_CANNON,
            uid::SHIELD_BATTERY,
            uid::HATCHERY,
            uid::LAIR,
            uid::HIVE,
            uid::EXTRACTOR,
            uid::SPIRE,
            uid::GREATER_SPIRE,
            uid::QUEENS_NEST,
            uid::NYDUS_CANAL,
            uid::SUNKEN_COLONY,
            uid::CREEP_COLONY,
        ] {
            let Some(path) = sprite_path(id) else {
                continue;
            };
            let Ok(bytes) = fs::read(asset_root.join("unit").join(path)) else {
                continue;
            };
            if let Some(grp) = Grp::parse(&bytes) {
                grps.insert(id, grp);
            }
        }
        Self { grps, palette: pal }
    }

    pub fn has(&self, type_id: u16) -> bool {
        self.grps.contains_key(&type_id)
    }

    pub fn gpr(&self, type_id: u16) -> Option<&Grp> {
        self.grps.get(&type_id)
    }

    pub fn palette(&self) -> &Palette {
        &self.palette
    }

    /// Draw with the map's terrain palette.  Retail draws SD units with the
    /// tileset palette (`tileset\<era>.wpe`); the generic `unit.pal` has
    /// placeholder blue in the tileset-dependent entries (203..=245 and
    /// friends), which showed as a dotted blue outline on the Siege Tank.
    pub fn set_palette(&mut self, palette: Palette) {
        self.palette = palette;
    }

    /// Draw a unit sprite.  Returns false when the unit has no bundled GRP.
    pub fn draw_unit(
        &self,
        fb: &mut Framebuffer,
        unit: &SimUnit,
        screen: (i32, i32),
        tick: u64,
    ) -> bool {
        self.draw_unit_posed(fb, unit, screen, tick, None)
    }

    /// [`Self::draw_unit`] with the unit's iscript pose (`playfram` operand
    /// from `crate::unit_anim`), so the SD view walks and attacks like the
    /// HD one.
    pub fn draw_unit_posed(
        &self,
        fb: &mut Framebuffer,
        unit: &SimUnit,
        screen: (i32, i32),
        tick: u64,
        pose: Option<u16>,
    ) -> bool {
        self.draw_unit_turned(fb, unit, screen, tick, pose, unit.facing)
    }

    /// [`Self::draw_unit_posed`] with a render-only facing, used by the
    /// Video option "Smooth Unit Turning": the simulation keeps its exact
    /// byte facing while the drawn sprite turns through the intermediate
    /// GRP directions.
    pub fn draw_unit_turned(
        &self,
        fb: &mut Framebuffer,
        unit: &SimUnit,
        screen: (i32, i32),
        tick: u64,
        pose: Option<u16>,
        facing: u8,
    ) -> bool {
        let Some(grp) = self.grps.get(&unit.type_id) else {
            return false;
        };
        let index = if matches!(unit.type_id, 0xcd..=0xd0) {
            if unit.doodad_enabled == Some(false) {
                grp.frames.len().saturating_sub(1)
            } else {
                0
            }
        } else {
            sd_frame_index(grp.frames.len(), facing, pose, !unit.path.is_empty(), tick)
        };
        let Some(frame) = grp.frame(index) else {
            return false;
        };
        if frame.w == 0 || frame.h == 0 {
            return false;
        }
        let team = Some(PLAYER_COLORS[unit.owner as usize % PLAYER_COLORS.len()]);
        let flip = !matches!(unit.type_id, 0xcd..=0xd0) && sd_frame_flip(grp.frames.len(), facing);
        draw_grp_posed(
            fb,
            grp,
            index,
            &self.palette,
            screen,
            flip,
            GrpInk::Solid(team),
        );
        true
    }

    pub fn draw_health(
        &self,
        fb: &mut Framebuffer,
        unit: &SimUnit,
        data: &GameData,
        screen: (i32, i32),
    ) {
        let ty = data.unit(unit.type_id);
        let max = ty.hit_points_raw as i32;
        if max <= 0 {
            return;
        }
        let w = ((ty.dimensions.right - ty.dimensions.left) as i32 / 2).clamp(16, 64);
        let hp = (unit.hp as i64 * w as i64 / max as i64).clamp(0, w as i64) as i32;
        let y = screen.1 - ((ty.dimensions.bottom - ty.dimensions.top) as i32 / 2) - 8;
        fb.rect(screen.0 - w / 2, y, w, 3, Rgb::new(20, 20, 20));
        let color = if unit.owner == 0 {
            Rgb::new(60, 220, 60)
        } else {
            Rgb::new(220, 60, 60)
        };
        fb.rect(screen.0 - w / 2, y, hp, 3, color);
        if unit.shields > 0 {
            let sw = (unit.shields as i64 * w as i64 / (ty.shield_points as i64 * 256).max(1))
                .clamp(0, w as i64) as i32;
            fb.rect(screen.0 - w / 2, y - 3, sw, 2, Rgb::new(80, 160, 255));
        }
    }
}

/// Draw a fallback box for units without a bundled sprite (buildings, effects).
pub fn draw_unit_box(
    fb: &mut Framebuffer,
    unit: &SimUnit,
    data: &GameData,
    screen: (i32, i32),
    selected: bool,
) {
    let ty = data.unit(unit.type_id);
    let w = ((ty.dimensions.right - ty.dimensions.left) as i32).clamp(14, 96);
    let h = ((ty.dimensions.bottom - ty.dimensions.top) as i32).clamp(14, 96);
    let color = PLAYER_COLORS[unit.owner as usize % PLAYER_COLORS.len()];
    let fill = color.blend(Rgb::new(10, 10, 10), 150);
    fb.rect(screen.0 - w / 2, screen.1 - h / 2, w, h, fill);
    fb.rect_outline(screen.0 - w / 2, screen.1 - h / 2, w, h, color);
    let label = data.unit_name(unit.type_id);
    let short: String = label
        .split_whitespace()
        .filter_map(|w| w.chars().next())
        .take(3)
        .collect();
    fb.text_centered(screen.0, screen.1 - 4, &short, Rgb::new(240, 240, 240), 1);
    if selected {
        fb.circle_outline(
            screen.0,
            screen.1,
            (w.max(h) / 2 + 6).min(64),
            Rgb::new(80, 255, 80),
        );
    }
}

pub fn asset_root() -> PathBuf {
    if let Ok(p) = std::env::var("GALAXYCRAFT_ASSETS") {
        return PathBuf::from(p);
    }
    // Prefer the crate-relative assets directory, then the working directory.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
    if manifest.exists() {
        manifest
    } else {
        PathBuf::from("assets")
    }
}

#[cfg(test)]
mod minimap_blip_tests {
    use super::{MinimapDot, minimap_blip_tiles};

    fn dot(x: i32, y: i32, placement: (i32, i32), building: bool) -> MinimapDot {
        MinimapDot {
            x,
            y,
            owner: 0,
            placement,
            building,
            resource: false,
            color: super::PLAYER_COLORS[0],
        }
    }

    #[test]
    fn blips_follow_the_placement_box_in_whole_tiles() {
        // Geyser 128x64 centred at (1000, 3000): 4 x 2 tiles at (29, 92).
        assert_eq!(
            minimap_blip_tiles(&dot(1000, 3000, (128, 64), true)),
            (29, 92, 4, 2)
        );
        // Mineral field 64x32: at least 2 x 2 (OpenBW, and 135051's rows).
        let mineral = minimap_blip_tiles(&dot(1000, 3000, (64, 32), false));
        assert_eq!((mineral.2, mineral.3), (2, 2));
        // Nexus 128x96: 4 x 3; buildings are clamped to 4 tiles.
        assert_eq!(minimap_blip_tiles(&dot(1000, 3000, (128, 96), true)).2, 4);
        assert_eq!(minimap_blip_tiles(&dot(1000, 3000, (128, 96), true)).3, 3);
        assert_eq!(minimap_blip_tiles(&dot(1000, 3000, (192, 160), true)).2, 4);
        // Units are not clamped.
        assert_eq!(minimap_blip_tiles(&dot(1000, 3000, (192, 160), false)).2, 6);
    }
}

#[cfg(test)]
mod view_scale_tests {
    use super::{Framebuffer, Rgb, box_blur, downsample_rgba, stretch_columns};
    use crate::dds::DdsImage;

    #[test]
    fn the_853_px_view_is_stretched_across_the_2560_px_box() {
        // 2559 drawn columns (853 game px at 3x): black, then white from
        // source x 2000 on; column 2559 was never drawn.
        let mut fb = Framebuffer::new(2560, 1);
        for x in 0..2559 {
            let v = if x < 2000 { 0 } else { 255 };
            fb.put(x, 0, Rgb::new(v, v, v));
        }
        fb.put(2559, 0, Rgb::new(255, 0, 0));
        stretch_columns(&mut fb, (0, 0, 2560, 1), 2559);
        let at = |x: i32| fb.get(x, 0);
        // The edge moves ~0.78 px right (retail drift measured on the
        // terrain: 0.95 px at x 2200, 1.00 at 2360): output 2000 samples
        // source 2000.5 * 2559 / 2560 - 0.5 = 1999.22 (a 22 % blend, 56)
        // instead of the white texel; the 50 % point moves to 2000.28.
        assert_eq!(at(1999), Rgb::new(0, 0, 0));
        assert_eq!(at(2000), Rgb::new(56, 56, 56));
        assert_eq!(at(2001), Rgb::new(255, 255, 255));
        // The undrawn column is replaced by the last drawn one.
        assert_eq!(at(2559), Rgb::new(255, 255, 255));
        // Equal widths are a no-op.
        let mut same = Framebuffer::new(4, 1);
        same.put(1, 0, Rgb::new(9, 9, 9));
        stretch_columns(&mut same, (0, 0, 4, 1), 4);
        assert_eq!(same.get(1, 0), Rgb::new(9, 9, 9));
    }

    fn image(w: usize, h: usize, px: &[u32]) -> DdsImage {
        DdsImage {
            width: w,
            height: h,
            pixels: px.to_vec(),
        }
    }

    #[test]
    fn hd_tiles_are_area_filtered_with_straight_alpha_out() {
        // 4 texels -> 3 px (the 128 -> 96 ratio): coverage 1 + 1/3,
        // 2/3 + 2/3, 1/3 + 1.  The old integer box gave 0, 255, 127.
        let g = |v: u32| 0xff00_0000 | (v << 16) | (v << 8) | v;
        let img = image(4, 1, &[g(0), g(255), g(0), g(255)]);
        let out = downsample_rgba(&img, 3, 1);
        let red = |p: u32| (p >> 16) & 0xff;
        // The middle pixel is exactly 127.5, so allow either rounding.
        let got: Vec<i32> = out.iter().map(|&p| red(p) as i32).collect();
        for (g, want) in got.iter().zip([64, 128, 191]) {
            assert!((g - want).abs() <= 1, "{got:?}");
        }
        assert!(out.iter().all(|&p| p >> 24 == 255));
        // A half-transparent texel keeps its colour (blit_tile blends with
        // straight alpha); the old path stored the premultiplied 100.
        let img = image(1, 1, &[(128 << 24) | (200 << 16) | (200 << 8) | 200]);
        let out = downsample_rgba(&img, 1, 1);
        assert_eq!((out[0] >> 24, red(out[0])), (128, 200));
    }

    #[test]
    fn the_fog_edge_is_a_linear_ramp() {
        // Step from 0 to 255 between samples 19 and 20 of a 40-wide row.
        let (w, h) = (40usize, 1usize);
        let mut v: Vec<f32> = (0..w).map(|x| if x < 20 { 0.0 } else { 255.0 }).collect();
        box_blur(&mut v, w, h, 5);
        // 11-sample ramp: 0 up to x 14, then equal steps of 255 / 11.
        assert_eq!(v[14], 0.0);
        let step = 255.0 / 11.0;
        for x in 15..25 {
            assert!((v[x] - v[x - 1] - step).abs() < 1e-3, "x {x}");
        }
        assert!((v[25] - 255.0).abs() < 1e-3);
    }
}

#[cfg(test)]
mod hd_terrain_tests {
    use super::HdTerrain;

    /// One 4x4 DXT1 record whose single block is solid red.
    fn record() -> Vec<u8> {
        let mut dds = b"DDS ".to_vec();
        let mut hdr = [0u8; 124];
        hdr[0] = 124;
        hdr[8..12].copy_from_slice(&4u32.to_le_bytes()); // height
        hdr[12..16].copy_from_slice(&4u32.to_le_bytes()); // width
        hdr[72] = 32; // pixel format size
        hdr[76] = 4; // DDPF_FOURCC
        hdr[80..84].copy_from_slice(b"DXT1");
        dds.extend_from_slice(&hdr);
        dds.extend_from_slice(&[0x00, 0xf8, 0x00, 0xf8, 0, 0, 0, 0]);
        let mut r = Vec::new();
        r.extend_from_slice(&0u32.to_le_bytes());
        r.extend_from_slice(&4u16.to_le_bytes());
        r.extend_from_slice(&4u16.to_le_bytes());
        r.extend_from_slice(&(dds.len() as u32).to_le_bytes());
        r.extend_from_slice(&dds);
        r
    }

    #[test]
    fn minimap_box_follows_the_classic_grid() {
        // 134905: 384 px minimap at x 18, y 1044, 128-tile map, camera
        // (1936, 112) + 853 x 368 view -> inner box 198..279 x 1053..1089,
        // 3 px stroke (measured edges 195/281 and 1050/1091).
        let b = super::minimap_view_box(
            (18, 384),
            1044,
            (128, 128),
            (1936, 112, 1936 + 853, 112 + 368),
        );
        assert_eq!(b, (198, 1053, 81, 36, 3));
    }

    #[test]
    fn creep_picker_is_the_crt_rand_rule_of_the_retail_client() {
        // MSVC `srand(1)` yields 41, 18467, 6334, 26500: the first pick
        // rolls 41 (common tile) with 18467 % 6 = 5, the second 34 with
        // 26500 % 6 = 4.
        use super::{CreepTiles, CrtRand};
        let mut r = CrtRand::new(1);
        assert_eq!(CreepTiles::pick(&mut r), 0x15);
        assert_eq!(CreepTiles::pick(&mut r), 0x14);
        // 4 % of picks are the speckled tiles 6..=12, the rest 0..=5.
        let mut r = CrtRand::new(12345);
        let n = 100_000;
        let rare = (0..n)
            .map(|_| CreepTiles::pick(&mut r))
            .inspect(|&t| assert!((0x10..=0x1c).contains(&t)))
            .filter(|&t| t >= 0x16)
            .count();
        assert!((3_500..4_500).contains(&rare), "{rare} rare tiles");
    }

    #[test]
    fn creep_tiles_pick_once_per_gain_and_forget_on_loss() {
        let mut c = super::CreepTiles::default();
        let r = &mut super::CrtRand::new(1);
        let mut grid = vec![0u8; 6];
        grid[4] = 3;
        c.update(r, &grid, 3);
        assert_eq!(c.get(1, 1), Some(0x15));
        assert_eq!(c.get(0, 0), None);
        // Maturity changes do not re-pick; a new tile draws the next pair.
        grid[4] = 1;
        grid[0] = 2;
        c.update(r, &grid, 3);
        assert_eq!((c.get(1, 1), c.get(0, 0)), (Some(0x15), Some(0x14)));
        // Losing creep forgets the tile; regaining it picks afresh.
        grid[4] = 0;
        c.update(r, &grid, 3);
        assert_eq!(c.get(1, 1), None);
        grid[4] = 3;
        c.update(r, &grid, 3);
        assert!(c.get(1, 1).is_some());
        assert_eq!(c.get(3, 0), None, "outside the map width");
    }

    #[test]
    fn creep_edges_only_surround_creep() {
        let creep = |x: i32, y: i32| (x == 5 && y == 5).then_some(0x10);
        assert_eq!(super::creep_edge_frame(&creep, 5, 5), None, "creep tile");
        assert_eq!(super::creep_edge_frame(&creep, 9, 9), None, "far away");
        let left = super::creep_edge_frame(&creep, 4, 5).expect("edge left of creep");
        let right = super::creep_edge_frame(&creep, 6, 5).expect("edge right of creep");
        assert_ne!(left, right);
    }

    #[test]
    fn creep_edge_table_matches_the_retail_frame_set() {
        let t = super::creep_edge_frame_table();
        assert_eq!(t[0], 0, "no neighbouring creep: no edge");
        // 37 frames in tileset/<era>.grp, numbered 1..=37 here.
        assert_eq!(*t.iter().max().unwrap(), 37);
        // Creep on one side draws an edge; the diagonals refine the frame.
        assert_ne!(t[0b10], 0);
        assert_ne!(t[0b10], t[0b111]);
    }

    #[test]
    fn archive_index_skips_the_eight_byte_header() {
        let mut bytes = vec![0u8; 8];
        bytes[4] = 2;
        bytes.extend(record());
        bytes.extend(record());
        let total = bytes.len() as u32;
        bytes[0..4].copy_from_slice(&total.to_le_bytes());
        let t = HdTerrain::parse(bytes).expect("index");
        assert_eq!(t.len(), 2);
        let px = t.tile(1, 2).expect("decode");
        assert_eq!(px.len(), 4);
        // Solid red, scaled by the measured terrain gain (255 * 0.91).
        assert_eq!(px[0] & 0x00ff_ffff, 0x00e8_0000, "red after resample");
        assert!(HdTerrain::parse(vec![0u8; 8]).is_none());
    }
}

#[cfg(test)]
mod par_bands_tests {
    use super::par_bands;

    #[test]
    fn every_row_is_visited_once_with_its_frame_index() {
        let (w, h) = (7usize, 1000usize);
        let mut px = vec![0u32; w * h];
        par_bands(&mut px, w, |first, band| {
            for (r, row) in band.chunks_mut(w).enumerate() {
                for p in row {
                    *p += (first + r) as u32 + 1;
                }
            }
        });
        for (y, row) in px.chunks(w).enumerate() {
            assert!(row.iter().all(|&p| p == y as u32 + 1), "row {y}");
        }
    }
}

#[cfg(test)]
mod sd_pose_tests {
    use super::{Rgb, color_cycle_ranges, rotate_palette, sd_frame_index, water_tint};

    #[test]
    fn the_sd_view_uses_the_iscript_pose_when_it_has_one() {
        // A 17 x 14 marine-like sheet facing up (direction 0).
        assert_eq!(
            sd_frame_index(238, 0, Some(34), false, 0),
            34,
            "attack pose"
        );
        assert_eq!(
            sd_frame_index(238, 128, Some(34), false, 0),
            // Facing 128 (south) is direction 16, the last unmirrored frame.
            34 + 16,
            "facing down"
        );
        // West (192) is direction 24: frame 32 - 24 = 8, mirrored.
        assert_eq!(sd_frame_index(238, 192, Some(34), false, 0), 34 + 8);
        assert!(super::sd_frame_flip(238, 192) && !super::sd_frame_flip(238, 64));
        assert!(!super::sd_frame_flip(4, 192), "static GRPs never mirror");
        assert_eq!(
            sd_frame_index(238, 0, None, false, 7),
            0,
            "idle without a pose"
        );
        assert_eq!(
            sd_frame_index(238, 0, Some(400), false, 0),
            0,
            "out of range pose ignored"
        );
        assert_eq!(sd_frame_index(4, 0, Some(34), true, 3), 0, "static GRP");
    }

    #[test]
    fn hd_megatiles_mark_their_colour_cycled_water() {
        use crate::map::Scenario;
        let raw = std::fs::read("assets/maps/(2)Challenger.chk").expect("bundled map");
        let sc = Scenario::parse(&raw).expect("parse");
        let ts = super::Tileset::load(std::path::Path::new("assets/TileSet"), sc.era, &sc)
            .expect("tileset");
        assert!(
            (0..ts.vx4.len() / 64).any(|m| ts.megatile_cycles(m)),
            "the badlands tileset has water megatiles"
        );
        let raw = std::fs::read("assets/maps/(2)Space Madness.chk").expect("platform map");
        let sc = Scenario::parse(&raw).expect("parse");
        let ts = super::Tileset::load(std::path::Path::new("assets/TileSet"), sc.era, &sc)
            .expect("tileset");
        assert!(
            !(0..ts.vx4.len() / 64).any(|m| ts.megatile_cycles(m)),
            "platform tilesets have no rotator set"
        );
        assert!(water_tint(4) > 1.0);
        assert!(water_tint(12) < 1.0);
        assert!((water_tint(0) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn color_cycle_rotates_the_retail_ranges() {
        let mut colors = [Rgb(0); 256];
        for (i, c) in colors.iter_mut().enumerate() {
            *c = Rgb::new(i as u8, 0, 0);
        }
        rotate_palette(&mut colors, 1, 6, 1);
        assert_eq!(colors[1], Rgb::new(6, 0, 0), "max wraps to min");
        assert_eq!(colors[2], Rgb::new(1, 0, 0));
        assert_eq!(colors[7], Rgb::new(7, 0, 0), "other ranges untouched");
        rotate_palette(&mut colors, 248, 254, 2);
        assert_eq!(colors[249], Rgb::new(254, 0, 0), "max wraps by two");
        assert_eq!(colors[250], Rgb::new(248, 0, 0));

        assert_eq!(
            color_cycle_ranges("badlands"),
            &[(1, 6), (7, 13), (248, 254)]
        );
        assert_eq!(color_cycle_ranges("ashworld"), &[(1, 4), (5, 8), (9, 13)]);
        assert_eq!(
            color_cycle_ranges("ice"),
            &[(1, 13), (248, 254)],
            "desert, ice and twilight share the long range"
        );
        assert!(color_cycle_ranges("platform").is_empty());
        assert!(color_cycle_ranges("installation").is_empty());
    }

    #[test]
    fn door_types_use_the_install_thingy_grps() {
        use crate::render::sprite_path;
        assert_eq!(
            sprite_path(0xcd),
            Some("unit/thingy/tileset/install/crdoor1.grp")
        );
        assert_eq!(
            sprite_path(0xce),
            Some("unit/thingy/tileset/install/crdoor1.grp")
        );
        assert_eq!(
            sprite_path(0xcf),
            Some("unit/thingy/tileset/install/didoor1.grp")
        );
        assert_eq!(
            sprite_path(0xd0),
            Some("unit/thingy/tileset/install/didoor1.grp")
        );
    }
}

// ===========================================================================
// Merged from src/wave13/agent01.rs
// ===========================================================================
// Wave 13 translation batch 01.
//
// Window `0x0065c000..0x0065dfff`: the `TilePage` family (`TilePage`,
// `TilePageDXT1`, `TilePagePaletteColor`) and the tile-set page cache that
// the terrain loader fills from `.tileset`/DDS data.
//
// Recovered layouts (all offsets are byte offsets, patch 1.23.10):
//
// `TilePage` (object size `0x44`, vtable `0x00d3d298`):
//
// | offset | type | meaning |
// |---|---|---|
// | `+0x00` | `u32` | vtable |
// | `+0x04` | `u32` | page index |
// | `+0x08` | `u32` | page kind (`0`, `1`, `2`; `2` writes the record frame slot) |
// | `+0x0c` | `u32` | tile edge length in bytes |
// | `+0x10` | `0x10` bytes | embedded base object destroyed by `FUN_00a892e0` |
// | `+0x14` | `i16` | page width / destination stride |
// | `+0x16` | `i16` | page height |
// | `+0x1c` | `u32` | unused by the translated page methods |
// | `+0x20` | pointer | pixel buffer (`std::vector<u8>` begin) |
// | `+0x24` | `u32` | pixel buffer length |
// | `+0x28` | `u32` | pixel buffer capacity, high bit = owned flag |
// | `+0x2c` | `u32` | cursor X |
// | `+0x30` | `u32` | cursor Y |
// | `+0x34` | `u32` | reference count (`0x0065c1f0` increments it) |
// | `+0x38` | `u32` | owner tag copied from the tile set (`+0x0c`) |
// | `+0x3c` | `u32` | bytes per pixel (`2` = DXT1, `4` = palette) |
// | `+0x40` | `u8` | sealed flag; a page is only reused while zero |
//
// `TileSet`:
//
// | offset | type | meaning |
// |---|---|---|
// | `+0x00` | array | page array, 8-byte `shared_ptr` entries |
// | `+0x04` | `u32` | page count |
// | `+0x08` | `u32` | page capacity, high bit = owned flag |
// | `+0x0c` | `u32` | owner tag copied to every page |
// | `+0x10` | array | 10-byte tile records |
// | `+0x14` | `u32` | tile-record count |
// | `+0x18` | `u32` | tile-record capacity |
// | `+0x1c` | `u32` | tile edge length |
// | `+0x20` | `u8` | non-zero: pages use DXT1 (2 bpp), else palette (4 bpp) |
// | `+0x24` | array | one presence byte per tile id |
// | `+0x28` | `u32` | presence length |
// | `+0x2c` | `u32` | presence capacity |
// | `+0x30` | `i16` | page width |
// | `+0x34` | `i16` | page height |
//
// A tile record is 10 bytes: `<u16 id><u16 page><u16 frame><u16 x><u16 y>`.
// New records are filled with `{0xfffe, 0xffff, 0xffff, 0, 0}`.
//
// The Ghidra ledger lists 58 rows in the window, but only 18 of them begin a
// standalone recoverable function.  Rows such as `FUN_0065c47c` and
// `FUN_0065c759` are basic blocks of `0x0065c410`; the remaining rows are
// listed by name in the "Skipped rows" table at the end.
//
// Verified against the shipped PE image at `0x0065c000..0x0065e000`; every
// translated address below appears in the function that owns that VA.
//
// Skipped rows (names only; none of these is claimed as a translation):
//
// | rows | reason |
// |---|---|
// | `FUN_0065c005`, `FUN_0065c028`, `FUN_0065c05f`, `FUN_0065c077`, `FUN_0065c095` | exception-unwind fragments of the 0x38-byte destructor that starts before this row; they begin mid-function, read `unaff_*`/`ExceptionList` and end in non-returning cleanup |
// | `FUN_0065c47c`, `FUN_0065c50e`, `FUN_0065c51d`, `FUN_0065c595`, `FUN_0065c61f`, `FUN_0065c68f`, `FUN_0065c71e`, `FUN_0065c759` | basic blocks of `0x0065c410` (shared_ptr/control-block construction, vector growth, record initialisation and tail zeroing); not standalone functions |
// | `FUN_0065c9f5` | basic block of `0x0065c960` (UV assembly after the texture-preparation call) |
// | `FUN_0065cb91`, `FUN_0065cc1a` | blocks of `0x0065cb10` (capacity growth and the `memset` tail) |
// | `FUN_0065cc30`, `FUN_0065cc46`, `FUN_0065cee3` | the `FUN_0065cc30` body is truncated at `FUN_0065cc46` and resumes after an embedded data/bad-instruction region split by exception funclets; no faithful whole function |
// | `FUN_0065cf60` | `halt_baddata` and overlapping instructions dominate the body |
// | `FUN_0065d150` | builds a `.tmsk` name with `std::string` internals and the unknown `FUN_0053f840`; no recoverable pure result |
// | `FUN_0065d410`, `FUN_0065d426`, `FUN_0065d4b7`, `FUN_0065d5c3`, `FUN_0065d60b`, `FUN_0065d656`, `FUN_0065d68c`, `FUN_0065d7c2`, `FUN_0065d805` | DDS tile loader and its blocks; virtual stream reads, caller-frame stack slots and vector-resize tails, no standalone pure result |
// | `FUN_0065d850` | configures 32x32 pages and then tail-calls `FUN_0060f870`/`FUN_00680190`; no recoverable return value |
// | `FUN_0065d972`, `FUN_0065da6d`, `FUN_0065daa0`, `FUN_0065dcec` | tile-image stream loader and its blocks; `unaff_*` registers/stack slots and a caller-owned stream vtable |
// | `FUN_0065dd40`, `FUN_0065de51`, `FUN_0065df74`, `FUN_0065df93`, `FUN_0065dfa5` | `halt_baddata`/overlapping instructions; decompile failed |

// ===========================================================================
// Tile record / page data
// ===========================================================================

/// Stride of one tile record (`0x0065c960` computes `id * 10`).
pub const W13A01_TILE_RECORD_STRIDE: usize = 10;

/// Tile id written into a fresh record (`0x0065c410` / the resizer behind
/// `FUN_0065e700` write `0xfffffffe` as the first dword, i.e. id `0xfffe`).
pub const W13A01_TILE_PLACEHOLDER_ID: u16 = 0xfffe;

/// "No tile / no page" sentinel (`0xffff`).
pub const W13A01_TILE_NONE: u16 = 0xffff;

/// Bytes-per-pixel tag stored at `TilePage + 0x3c` for DXT1 pages.
pub const W13A01_BPP_DXT1: u32 = 2;

/// Bytes-per-pixel tag stored at `TilePage + 0x3c` for palette pages.
pub const W13A01_BPP_PALETTE: u32 = 4;

/// Page kind whose index is written into the record's frame slot by
/// `0x0065c1f0`.
pub const W13A01_KIND_DXT1: u32 = 2;

/// `TilePage` vtable stored by the deleting destructor at `0x0065c170`.
pub const W13A01_TILE_PAGE_VTABLE: u32 = 0x00d3_d298;

/// Allocation size passed to `operator delete` by `0x0065c170`.
pub const W13A01_TILE_PAGE_OBJECT_SIZE: usize = 0x44;

/// `blz::_shared_ptr_manager` vtable stored by `0x0065c140`.
pub const W13A01_SHARED_PTR_MANAGER_VTABLE: u32 = 0x00d3_287c;

/// Allocation size passed to `operator delete` by `0x0065c140`.
pub const W13A01_SHARED_PTR_MANAGER_SIZE: usize = 0x50;

/// One 10-byte tile-set record.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13A01TileRecord {
    /// `+0x00`: tile id; the lookup at `0x0065c960` rejects a record whose
    /// stored id differs from the requested id.
    pub id: u16,
    /// `+0x02`: primary page index used by `0x0065c960`.
    pub page: u16,
    /// `+0x04`: secondary page index written when a page of kind
    /// [`W13A01_KIND_DXT1`] is selected.
    pub frame: u16,
    /// `+0x06`: tile X inside the page, written by both blitters.
    pub x: u16,
    /// `+0x08`: tile Y inside the page, written by both blitters.
    pub y: u16,
}

impl W13A01TileRecord {
    /// The value the record resizer writes into newly grown slots.
    pub const PLACEHOLDER: Self = Self {
        id: W13A01_TILE_PLACEHOLDER_ID,
        page: W13A01_TILE_NONE,
        frame: W13A01_TILE_NONE,
        x: 0,
        y: 0,
    };

    /// Decode one little-endian record.
    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let b: [u8; W13A01_TILE_RECORD_STRIDE] =
            bytes.get(..W13A01_TILE_RECORD_STRIDE)?.try_into().ok()?;
        Some(Self {
            id: u16::from_le_bytes([b[0], b[1]]),
            page: u16::from_le_bytes([b[2], b[3]]),
            frame: u16::from_le_bytes([b[4], b[5]]),
            x: u16::from_le_bytes([b[6], b[7]]),
            y: u16::from_le_bytes([b[8], b[9]]),
        })
    }

    /// Encode one little-endian record; returns `false` when `out` is short.
    pub fn write_bytes(self, out: &mut [u8]) -> bool {
        if out.len() < W13A01_TILE_RECORD_STRIDE {
            return false;
        }
        out[0..2].copy_from_slice(&self.id.to_le_bytes());
        out[2..4].copy_from_slice(&self.page.to_le_bytes());
        out[4..6].copy_from_slice(&self.frame.to_le_bytes());
        out[6..8].copy_from_slice(&self.x.to_le_bytes());
        out[8..10].copy_from_slice(&self.y.to_le_bytes());
        true
    }
}

/// One tile page.  The `pixels` vector stands for the `+0x20..+0x28`
/// `std::vector<u8>` inside the shipped object.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W13A01TilePage {
    pub kind: u32,
    pub index: u16,
    pub tile_size: u32,
    pub width: i16,
    pub height: i16,
    pub pixels: Vec<u8>,
    pub cursor_x: u32,
    pub cursor_y: u32,
    pub refs: u32,
    pub owner_tag: u32,
    pub bpp: u32,
    pub sealed: bool,
}

impl W13A01TilePage {
    /// Virtual `size()` of the page: DXT1 halves the area, palette keeps it.
    pub fn size_bytes(&self) -> u32 {
        if self.bpp == W13A01_BPP_DXT1 {
            w13a01_dxt1_page_size(self.width, self.height)
        } else {
            w13a01_palette_page_size(self.width, self.height)
        }
    }
}

/// The tile set object (page array + record table + presence map).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W13A01Tileset {
    pub pages: Vec<W13A01TilePage>,
    pub field_04: u32,
    pub field_08: u32,
    pub owner_tag: u32,
    pub records: Vec<W13A01TileRecord>,
    pub tile_size: u32,
    pub use_dxt1: bool,
    pub present: Vec<u8>,
    pub width: i16,
    pub height: i16,
}

/// Result of the tile lookup at `0x0065c960`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct W13A01TileUv {
    /// Record `+0x02` (the primary page index).
    pub page: u16,
    /// Record `+0x04`.
    pub frame: u16,
    /// `x / page.width`.
    pub u0: f32,
    /// `y / page.height`.
    pub v0: f32,
    /// `tile_size / page.width`.
    pub u1: f32,
    /// `tile_size / page.height`.
    pub v1: f32,
}

// ===========================================================================
// Page geometry and blitters
// ===========================================================================

/// `0x0065c940`: `TilePageDXT1::size()`.
///
/// ```c
/// return (short[+0x16] * short[+0x14]) >> 1;
/// ```
///
/// The two operand loads are sign-extending `movsx` and the multiply is a
/// 32-bit `imul`; the shift is logical.  The result is a byte count.
pub fn w13a01_dxt1_page_size(width: i16, height: i16) -> u32 {
    ((width as i32).wrapping_mul(height as i32) as u32) >> 1
}

/// `0x0065c950`: `TilePagePaletteColor::size()`.
///
/// ```c
/// return (short[+0x16] * short[+0x14]);
/// ```
///
/// Same sign-extending 32-bit multiply as [`w13a01_dxt1_page_size`], without
/// the halving step.
pub fn w13a01_palette_page_size(width: i16, height: i16) -> u32 {
    (width as i32).wrapping_mul(height as i32) as u32
}

/// `0x0065cf00`: `TilePageDXT1::is_full()`.
///
/// ```c
/// cols = (uint)(int)width / tile_size;          // unsigned div
/// rows = (height + ((height >> 31) & 3)) >> 2;  // truncating / 4
/// return cursor_y >= cols * rows;
/// ```
///
/// A zero `tile_size` would fault in the shipped code; this version reports
/// "not full" instead.
pub fn w13a01_dxt1_is_full(width: i16, height: i16, tile_size: u32, cursor_y: u32) -> bool {
    let cols = match (width as i32 as u32).checked_div(tile_size) {
        Some(v) => v,
        None => return false,
    };
    let h = height as i32;
    let rows = (h.wrapping_add((h >> 31) & 3) >> 2) as u32;
    cursor_y >= cols.wrapping_mul(rows)
}

/// `0x0065cf30`: `TilePagePaletteColor::is_full()`.
///
/// ```c
/// return cursor_y >= (uint)(int)height;
/// ```
///
/// Unlike the DXT1 test this compares directly with the page height; the
/// palette page is full only after the last 1-byte row has been consumed.
pub fn w13a01_palette_is_full(height: i16, cursor_y: u32) -> bool {
    cursor_y >= height as i32 as u32
}

/// `0x0065c7a0`: `TilePageDXT1::blit(record, source)`.
///
/// The shipped body copies `tile_size / 4` rows of `(tile_size / 4) * 8`
/// bytes, advancing the destination row by `width / tile_size` each time, then
/// writes the tile's page coordinates to `record.x`/`record.y` and advances the
/// cursor (`cursor_x += stride`; wrap to the next band when the band is full).
///
/// The destination offset is `(cursor_y + local) * stride + cursor_x`; the
/// original has no bounds checks.  This version returns `false` without
/// writing when a row would leave the pixel buffer or the source slice, and
/// uses zero for the coordinate divisions a zero-sized page would fault on.
pub fn w13a01_dxt1_blit(
    page: &mut W13A01TilePage,
    record: &mut W13A01TileRecord,
    source: &[u8],
) -> bool {
    let tile = page.tile_size;
    if tile == 0 {
        return false;
    }
    let rows = tile >> 2;
    // The shipped body forms the row stride as `(tile_size >> 2) * 8`, which
    // only equals `tile_size * 2` for tile sizes that are multiples of four.
    let stride = rows.wrapping_mul(8);
    if rows == 0 || stride == 0 {
        return false;
    }
    let row_step = match (page.width as i32 as u32).checked_div(tile) {
        Some(v) => v,
        None => return false,
    };
    let band_height = row_step.wrapping_mul(rows);
    if band_height == 0 {
        return false;
    }

    let mut local = 0u32;
    let mut src_off = 0usize;
    for _ in 0..rows {
        let dst_start =
            (page.cursor_y as u64 + local as u64) * stride as u64 + page.cursor_x as u64;
        let dst_end = dst_start + stride as u64;
        let src_end = src_off as u64 + stride as u64;
        if dst_end > page.pixels.len() as u64 || src_end > source.len() as u64 {
            return false;
        }
        page.pixels[dst_start as usize..dst_end as usize]
            .copy_from_slice(&source[src_off..src_end as usize]);
        src_off = src_end as usize;
        local = local.wrapping_add(row_step);
    }

    let tile_low = tile as u16 as u32;
    record.x = (page.cursor_x / stride).wrapping_mul(tile_low) as u16;
    record.y = (page.cursor_y / band_height).wrapping_mul(tile_low) as u16;

    page.cursor_x = page.cursor_x.wrapping_add(stride);
    if band_height.wrapping_mul(stride) as u64 <= page.cursor_x as u64 {
        page.cursor_y = page.cursor_y.wrapping_add(band_height);
        page.cursor_x = 0;
    }
    true
}

/// `0x0065c880`: `TilePagePaletteColor::blit(record, source)`.
///
/// Copies a `tile_size` x `tile_size` square of bytes into
/// `pixels[(cursor_y + dy) * width + cursor_x + dx]`, stores the cursor in the
/// record's X/Y slots and advances `cursor_x` by one tile; when it reaches the
/// page width the cursor wraps to the next tile row.
///
/// The shipped body also has no bounds checks; this version returns `false`
/// when a destination byte or source byte is out of range.
pub fn w13a01_palette_blit(
    page: &mut W13A01TilePage,
    record: &mut W13A01TileRecord,
    source: &[u8],
) -> bool {
    let edge = page.tile_size;
    if edge == 0 {
        return false;
    }
    let width = page.width as i32;
    let mut src_off = 0usize;
    for dy in 0..edge {
        for dx in 0..edge {
            let row = (page.cursor_y as i64 + dy as i64) * width as i64
                + page.cursor_x as i64
                + dx as i64;
            if row < 0 || src_off >= source.len() {
                return false;
            }
            let dst = row as usize;
            if dst >= page.pixels.len() {
                return false;
            }
            page.pixels[dst] = source[src_off];
            src_off += 1;
        }
    }

    record.x = page.cursor_x as u16;
    record.y = page.cursor_y as u16;
    page.cursor_x = page.cursor_x.wrapping_add(edge);
    if page.cursor_x >= width as u32 {
        page.cursor_y = page.cursor_y.wrapping_add(edge);
        page.cursor_x = 0;
    }
    true
}

/// Dispatch the virtual `size()` (slots 1 of both page vtables).
pub fn w13a01_page_size(page: &W13A01TilePage) -> u32 {
    page.size_bytes()
}

/// Dispatch the virtual `is_full()` (slot 2 of both page vtables).
pub fn w13a01_page_is_full(page: &W13A01TilePage) -> bool {
    if page.bpp == W13A01_BPP_DXT1 {
        w13a01_dxt1_is_full(page.width, page.height, page.tile_size, page.cursor_y)
    } else {
        w13a01_palette_is_full(page.height, page.cursor_y)
    }
}

/// Dispatch the virtual `blit(record, source)` (slot 3 of both page vtables).
pub fn w13a01_page_blit(
    page: &mut W13A01TilePage,
    record: &mut W13A01TileRecord,
    source: &[u8],
) -> bool {
    if page.bpp == W13A01_BPP_DXT1 {
        w13a01_dxt1_blit(page, record, source)
    } else {
        w13a01_palette_blit(page, record, source)
    }
}

// ===========================================================================
// Tile-set accessors
// ===========================================================================

/// `0x0065c910`: the number of pages in the tile-set object behind the
/// wrapper.
///
/// ```c
/// return *(u32 *)(*in_ECX + 4);
/// ```
///
/// `this->[0]` is a `TileSet *`; `TileSet + 0x04` is the live page count,
/// i.e. the length of the page vector whose begin/capacity words sit at
/// `+0x00`/`+0x08`.
pub fn w13a01_tileset_page_count(tileset: &W13A01Tileset) -> u32 {
    tileset.pages.len() as u32
}

/// `0x0065c920`: the tile edge length divided by the wrapper's slot 9.
///
/// ```c
/// v = *(u32 *)(*in_ECX + 0x1c);
/// if (in_ECX[9] != 0) v /= in_ECX[9];
/// return v;
/// ```
///
/// `in_ECX[9]` is the dword at wrapper offset `+0x24`.  A zero divisor keeps
/// the value unchanged, matching the shipped `test ecx,ecx; je` guard.
pub fn w13a01_tileset_scaled_tile_size(tileset: &W13A01Tileset, divisor: u32) -> u32 {
    tileset
        .tile_size
        .checked_div(divisor)
        .unwrap_or(tileset.tile_size)
}

/// `0x0065cb00`: the tile edge length (`TileSet + 0x1c`).
///
/// ```c
/// return *(u32 *)(*in_ECX + 0x1c);
/// ```
pub fn w13a01_tileset_tile_size(tileset: &W13A01Tileset) -> u32 {
    tileset.tile_size
}

/// `0x0065cf40`: the per-tile presence byte.
///
/// ```c
/// return *(u8 *)(*(u32 *)(*in_ECX + 0x24) + param_1);
/// ```
///
/// The presence map is the one resized by [`w13a01_configure_tile_set`].  The
/// shipped read is unchecked; this version returns `None` past the map end.
pub fn w13a01_tile_presence(tileset: &W13A01Tileset, tile_id: u16) -> Option<u8> {
    tileset.present.get(tile_id as usize).copied()
}

/// `0x0065caa0`: resolve the page stored at `index` in the tile set's page
/// array.
///
/// ```c
/// page = (*(TilePage ***)set)[index];
/// if (!prepare_texture(page + 0x10)) { lock_surface(page + 0x10); ... }
/// return page + 4;                 // address of the +0x10 subobject
/// ```
///
/// The shipped return value is the embedded base object at `TilePage + 0x10`;
/// this helper returns the owning [`W13A01TilePage`] instead.  The texture
/// preparation (`FUN_0067acc0`, `FUN_00a894d0`, `FUN_00a89790`) is a side effect and
/// is not represented.  Out-of-range indices return `None` instead of reading
/// past the array.
pub fn w13a01_tile_page_at(tileset: &W13A01Tileset, index: u16) -> Option<&W13A01TilePage> {
    tileset.pages.get(index as usize)
}

/// `0x0065cb10`: configure the tile set and resize its two parallel tables.
///
/// ```c
/// this->[+0x0c] = param_3;      // owner tag
/// this->[+0x1c] = param_2;      // tile edge
/// this->[+0x20] = param_4;      // DXT1 flag
/// resize_records(param_1);      // FUN_0065e700: new slots get 0xfffe/0xffff
/// resize_present(param_1);      // new bytes are zeroed
/// ```
///
/// `FUN_0065e700` fills fresh 10-byte records with
/// [`W13A01TileRecord::PLACEHOLDER`](W13A01TileRecord::PLACEHOLDER); the
/// presence vector is grown with `memset(..., 0, ...)`.
pub fn w13a01_configure_tile_set(
    tileset: &mut W13A01Tileset,
    count: u32,
    tile_size: u32,
    owner_tag: u32,
    use_dxt1: bool,
) {
    tileset.owner_tag = owner_tag;
    tileset.tile_size = tile_size;
    tileset.use_dxt1 = use_dxt1;
    tileset
        .records
        .resize(count as usize, W13A01TileRecord::PLACEHOLDER);
    tileset.present.resize(count as usize, 0);
}

/// `0x0065c960`: look up a tile record and build its texture UV rectangle.
///
/// ```c
/// rec = record_base + tile_id * 10;
/// if (rec->id != tile_id) return 0;
/// *out_page = rec->page;
/// if (out_frame) *out_frame = rec->frame;
/// page = pages[rec->page];
/// tile = (float)set->tile_size;
/// rect = { rec->x / page->width, rec->y / page->height,
///          tile / page->width, tile / page->height };
/// ```
///
/// The shipped body additionally prepares the page texture through
/// `FUN_0067acc0`/`FUN_00a894d0`/`FUN_00a89790` before reading the dimensions; that
/// side effect is not part of this value helper.  A zero page dimension is
/// rejected here instead of dividing by zero.
pub fn w13a01_tile_uv(tileset: &W13A01Tileset, tile_id: u16) -> Option<W13A01TileUv> {
    let record = *tileset.records.get(tile_id as usize)?;
    if record.id != tile_id {
        return None;
    }
    let page = tileset.pages.get(record.page as usize)?;
    let w = page.width as f32;
    let h = page.height as f32;
    if w == 0.0 || h == 0.0 {
        return None;
    }
    let tile = tileset.tile_size as f32;
    Some(W13A01TileUv {
        page: record.page,
        frame: record.frame,
        u0: record.x as f32 / w,
        v0: record.y as f32 / h,
        u1: tile / w,
        v1: tile / h,
    })
}

// ===========================================================================
// Page construction and cache
// ===========================================================================

/// `0x0065c410`: build one page, append it to the tile set's page array, and
/// size its pixel buffer.
///
/// The shipped body:
///
/// * takes `index = (u16)set->page_count` before the append;
/// * picks `TilePageDXT1` (2 bpp, vtable `0x00d3d2ac`) when
///   `set->[+0x20] != 0`, otherwise `TilePagePaletteColor` (4 bpp, vtable
///   `0x00d3d2c0`);
/// * stores `page->[+0x04] = index`, `page->[+0x08] = kind`,
///   `page->[+0x0c] = set->tile_size`, `page->[+0x14] = (i16)set->width`,
///   `page->[+0x16] = (i16)set->height`, `page->[+0x38] = set->[+0x0c]`,
///   `page->[+0x40] = 0`;
/// * appends the shared pointer and calls the virtual `size()` to resize the
///   pixel vector, zero-filling the new tail.
///
/// Returns the zero-based index of the new page.  The 0x50-byte
/// `_shared_ptr_emplace` control block is represented by the Rust value.
pub fn w13a01_new_tile_page(tileset: &mut W13A01Tileset, kind: u32) -> usize {
    let index = tileset.pages.len() as u16;
    let bpp = if tileset.use_dxt1 {
        W13A01_BPP_DXT1
    } else {
        W13A01_BPP_PALETTE
    };
    let mut page = W13A01TilePage {
        kind,
        index,
        tile_size: tileset.tile_size,
        width: tileset.width,
        height: tileset.height,
        pixels: Vec::new(),
        cursor_x: 0,
        cursor_y: 0,
        refs: 0,
        owner_tag: tileset.owner_tag,
        bpp,
        sealed: false,
    };
    let size = page.size_bytes() as usize;
    page.pixels.resize(size, 0);
    tileset.pages.push(page);
    index as usize
}

/// The page-selection loop of `0x0065c1f0`.
///
/// ```c
/// for (i = 0; i < set->page_count; ++i) {
///   page = set->pages[i];
///   if (page->kind == kind && !page->is_full() && page->[+0x40] == 0)
///     return i;
/// }
/// ```
///
/// The shared-pointer addref/release traffic around each iteration is not
/// modelled; the page's `refs` counter is incremented by
/// [`w13a01_place_tile`] exactly where the shipped body increments
/// `page->[+0x34]`.
pub fn w13a01_find_tile_page(tileset: &W13A01Tileset, kind: u32) -> Option<usize> {
    tileset
        .pages
        .iter()
        .position(|p| p.kind == kind && !p.sealed && !w13a01_page_is_full(p))
}

/// `0x0065c1f0`: find or create a page for `kind` and blit one tile into it.
///
/// The hit path increments `page->[+0x34]`, calls the virtual blit with
/// `(page->[+0x20], record, source)`, then writes `page->[+0x04]` into
/// `record->[+0x04]` when `page->[+0x08] == 2`, otherwise into
/// `record->[+0x02]`.  The miss path calls `0x0065c410`, then applies the same
/// `!is_full && !sealed` test to the new page.
///
/// Returns `false` only when the newly constructed page is already full or
/// sealed, matching the shipped `uVar8 = 0` exit.  A failed blit (out of
/// bounds in the safe model) still counts as a cache hit; the shipped blit
/// has no failure path.
pub fn w13a01_place_tile(
    tileset: &mut W13A01Tileset,
    kind: u32,
    record: &mut W13A01TileRecord,
    source: &[u8],
) -> bool {
    let page_index = match w13a01_find_tile_page(tileset, kind) {
        Some(index) => index,
        None => {
            let index = w13a01_new_tile_page(tileset, kind);
            let page = &tileset.pages[index];
            if page.sealed || w13a01_page_is_full(page) {
                return false;
            }
            index
        }
    };

    let page = &mut tileset.pages[page_index];
    page.refs = page.refs.wrapping_add(1);
    let _ = w13a01_page_blit(page, record, source);
    if page.kind == W13A01_KIND_DXT1 {
        record.frame = page.index;
    } else {
        record.page = page.index;
    }
    true
}

// ===========================================================================
// Destructors
// ===========================================================================

/// The two owned buffers and the third refcounted slot released by
/// `0x0065c0b0`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W13A01OwnedResources {
    pub first: Option<Vec<u8>>,
    pub second: Option<Vec<u8>>,
    pub third: Option<Vec<u8>>,
    /// `+0x10`: the third slot is released only while this is non-negative.
    pub third_state: i32,
}

/// `0x0065c0b0`: release the three resources owned by the object.
///
/// ```c
/// if (this->[0]) { free(this->[0]); this->[0] = 0; }
/// if (this->[1]) { free(this->[1]); this->[1] = 0; }
/// if (this->[4] >= 0) {
///   obj = this->[2];
///   obj->vtable[4]();       // release
///   this->[2] = 0;
/// }
/// ```
///
/// Dropping the `Vec`s models the two `free` calls; the negative-state test
/// is kept verbatim.
pub fn w13a01_release_owned_resources(resources: &mut W13A01OwnedResources) {
    resources.first = None;
    resources.second = None;
    if resources.third_state >= 0 {
        resources.third = None;
    }
}

/// The `_shared_ptr_manager` control block touched by `0x0065c140`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W13A01SharedPtrManager {
    pub vtable: u32,
}

/// `0x0065c140`: the scalar deleting destructor of `_shared_ptr_manager`.
///
/// ```c
/// *this = blz::_shared_ptr_manager::vftable;   // 0x00d3287c
/// if (flags & 1) operator delete(this, 0x50);
/// return this;
/// ```
///
/// Returns the allocation size the shipped code hands to `operator delete`,
/// or `None` when the deleting branch is not taken.
pub fn w13a01_shared_ptr_manager_deleting_dtor(
    manager: &mut W13A01SharedPtrManager,
    flags: u8,
) -> Option<usize> {
    manager.vtable = W13A01_SHARED_PTR_MANAGER_VTABLE;
    if flags & 1 != 0 {
        Some(W13A01_SHARED_PTR_MANAGER_SIZE)
    } else {
        None
    }
}

/// The `TilePage` state inspected by the deleting destructor at `0x0065c170`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W13A01TilePageDeletion {
    pub vtable: u32,
    /// `+0x20`: pixel/surface buffer released through the object's vtable.
    pub surface: Option<Vec<u8>>,
    /// `+0x28`: the release path runs only while this is non-negative.
    pub surface_state: i32,
    /// Stands for the embedded base object at `+0x10`, destroyed by
    /// `FUN_00a892e0`.
    pub base_live: bool,
}

/// `0x0065c170`: the scalar deleting destructor of `TilePage`.
///
/// ```c
/// *this = TilePage::vftable;                   // 0x00d3d298
/// if (this->[+0x28] >= 0) {
///   this->[+0x20]->vtable[4]();                // release surface
///   this->[+0x20] = 0;
/// }
/// destroy_base(this + 0x10);                   // FUN_00a892e0
/// if (flags & 1) operator delete(this, 0x44);
/// return this;
/// ```
///
/// Returns the allocation size for the deleting branch.
pub fn w13a01_tile_page_deleting_dtor(
    page: &mut W13A01TilePageDeletion,
    flags: u8,
) -> Option<usize> {
    page.vtable = W13A01_TILE_PAGE_VTABLE;
    if page.surface_state >= 0 {
        page.surface = None;
    }
    page.base_live = false;
    if flags & 1 != 0 {
        Some(W13A01_TILE_PAGE_OBJECT_SIZE)
    } else {
        None
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests_wave13_agent01 {
    use super::*;

    fn palette_tileset() -> W13A01Tileset {
        W13A01Tileset {
            tile_size: 8,
            width: 32,
            height: 32,
            use_dxt1: false,
            ..Default::default()
        }
    }

    #[test]
    fn record_layout_round_trips_and_placeholder_is_fffe() {
        let record = W13A01TileRecord {
            id: 0x1234,
            page: 0x0002,
            frame: 0x0003,
            x: 0x0040,
            y: 0x0080,
        };
        let mut bytes = [0u8; W13A01_TILE_RECORD_STRIDE];
        assert!(record.write_bytes(&mut bytes));
        assert_eq!(W13A01TileRecord::from_bytes(&bytes), Some(record));
        assert_eq!(bytes[0..2], [0x34, 0x12]);
        assert_eq!(bytes[8..10], [0x80, 0x00]);
        assert_eq!(W13A01TileRecord::from_bytes(&bytes[..9]), None);
        assert!(!record.write_bytes(&mut bytes[..9]));

        let mut placeholder = [0u8; 10];
        assert!(W13A01TileRecord::PLACEHOLDER.write_bytes(&mut placeholder));
        assert_eq!(&placeholder[0..2], &[0xfe, 0xff]);
        assert_eq!(&placeholder[2..6], &[0xff, 0xff, 0xff, 0xff]);
        assert_eq!(&placeholder[6..10], &[0, 0, 0, 0]);
    }

    #[test]
    fn page_sizes_match_dxt1_half_and_palette_area() {
        // 0x0065c940 / 0x0065c950
        assert_eq!(w13a01_dxt1_page_size(256, 128), 0x4000);
        assert_eq!(w13a01_dxt1_page_size(32, 32), 0x200);
        assert_eq!(w13a01_palette_page_size(256, 128), 0x8000);
        assert_eq!(w13a01_palette_page_size(32, 32), 0x400);
        // Signed operands: the shipped body sign-extends both halves, then
        // shifts the 32-bit product right logically.
        assert_eq!(w13a01_dxt1_page_size(-4, 8), 0x7fff_fff0);
        assert_eq!(w13a01_palette_page_size(-4, 8), 0xffff_ffe0);
    }

    #[test]
    fn dxt1_page_is_full_uses_width_over_tile_times_height_over_four() {
        // 0x0065cf00: cols = 64 / 8 = 8, rows = 32 / 4 = 8, need 64.
        assert!(!w13a01_dxt1_is_full(64, 32, 8, 63));
        assert!(w13a01_dxt1_is_full(64, 32, 8, 64));
        assert!(w13a01_dxt1_is_full(64, 32, 8, 65));
        // Truncating signed divide: -1 / 4 == 0 after the +3 bias.
        assert!(w13a01_dxt1_is_full(8, -1, 8, 0));
        // The shipped divide faults on a zero tile size; the model says no.
        assert!(!w13a01_dxt1_is_full(64, 32, 0, 999));
    }

    #[test]
    fn palette_page_is_full_compares_against_height() {
        // 0x0065cf30
        assert!(!w13a01_palette_is_full(32, 31));
        assert!(w13a01_palette_is_full(32, 32));
        assert!(w13a01_palette_is_full(32, 33));
        // A negative height sign-extends to 0xffffffff, so 0 is below it.
        assert!(!w13a01_palette_is_full(-1, 0));
    }

    #[test]
    fn palette_blit_copies_square_and_advances_cursor() {
        // 0x0065c880
        let mut page = W13A01TilePage {
            tile_size: 2,
            width: 5,
            height: 4,
            pixels: vec![0; 20],
            ..Default::default()
        };
        let mut record = W13A01TileRecord::default();
        let source = [1, 2, 3, 4];
        assert!(w13a01_palette_blit(&mut page, &mut record, &source));
        assert_eq!(page.pixels[0..2], [1, 2]);
        assert_eq!(page.pixels[5..7], [3, 4]);
        assert_eq!(record.x, 0);
        assert_eq!(record.y, 0);
        assert_eq!(page.cursor_x, 2);

        // The next tile wraps: 2 + 2 >= width 5 is false, so no wrap yet.
        assert!(w13a01_palette_blit(&mut page, &mut record, &[5, 6, 7, 8]));
        assert_eq!(page.cursor_x, 4);
        // Third tile crosses the right edge: 4 + 2 >= 5 -> new row.
        assert!(w13a01_palette_blit(
            &mut page,
            &mut record,
            &[9, 10, 11, 12]
        ));
        assert_eq!(record.x, 4);
        assert_eq!(page.cursor_x, 0);
        assert_eq!(page.cursor_y, 2);
        assert_eq!(page.pixels[4], 9);
        assert_eq!(page.pixels[5], 10);
        assert_eq!(page.pixels[9], 11);
        assert_eq!(page.pixels[10], 12);
        assert_eq!(page.pixels[8], 8);

        // Out-of-range destination: 2 rows at y=2 with height 4 works, y=3
        // would need row 4; force the cursor past the buffer.
        page.cursor_y = 99;
        assert!(!w13a01_palette_blit(&mut page, &mut record, &source));
    }

    #[test]
    fn dxt1_blit_copies_strips_and_writes_tile_coordinates() {
        // 0x0065c7a0 with tile=8: rows=2, stride=16, row_step=width/8=4,
        // band_height=8.  Destination row 0 at 0 and row 4 at 4*16=64.
        let mut page = W13A01TilePage {
            tile_size: 8,
            width: 32,
            height: 64,
            pixels: vec![0; 8 * 16],
            ..Default::default()
        };
        let mut record = W13A01TileRecord::default();
        let source: Vec<u8> = (0..32).collect();
        assert!(w13a01_dxt1_blit(&mut page, &mut record, &source));
        assert_eq!(&page.pixels[0..16], &source[0..16]);
        assert_eq!(&page.pixels[64..80], &source[16..32]);
        assert_eq!(record.x, 0);
        assert_eq!(record.y, 0);
        assert_eq!(page.cursor_x, 16);

        // cursor_x=16 -> x = 16/16 * 8 = 8; the next band starts at 8*16.
        assert!(w13a01_dxt1_blit(&mut page, &mut record, &source));
        assert_eq!(record.x, 8);
        assert_eq!(page.cursor_x, 32);
        // band_height * stride = 8*16 = 128; 32 < 128, no wrap yet.
        assert_eq!(page.cursor_y, 0);

        // Set the cursor at the end of the band: cursor_x 0, y 8 wraps after
        // the blit because the new cursor_x (16) is still below 128 -- so to
        // exercise the wrap use a narrow page: width 8 -> row_step 1.
        let mut narrow = W13A01TilePage {
            tile_size: 8,
            width: 8,
            height: 64,
            pixels: vec![0; 8 * 16],
            ..Default::default()
        };
        assert!(w13a01_dxt1_blit(&mut narrow, &mut record, &source));
        // band_height = 1 * 2 = 2; 2*16 = 32 <= cursor_x 16? No: 16 < 32.
        assert_eq!(narrow.cursor_y, 0);
        narrow.cursor_x = 16;
        assert!(w13a01_dxt1_blit(&mut narrow, &mut record, &source));
        assert_eq!(narrow.cursor_x, 0);
        assert_eq!(narrow.cursor_y, 2);

        // Short source -> no write.
        let mut fresh = W13A01TilePage {
            tile_size: 8,
            width: 32,
            height: 64,
            pixels: vec![0; 128],
            ..Default::default()
        };
        assert!(!w13a01_dxt1_blit(&mut fresh, &mut record, &[0u8; 31]));

        // Non-multiple-of-four tile size: tile=6 -> rows=1, stride=8 (not 12).
        let mut odd = W13A01TilePage {
            tile_size: 6,
            width: 24,
            height: 32,
            pixels: vec![0; 64],
            cursor_x: 16,
            cursor_y: 4,
            ..Default::default()
        };
        let mut odd_record = W13A01TileRecord::default();
        let eight = [7u8; 8];
        assert!(w13a01_dxt1_blit(&mut odd, &mut odd_record, &eight));
        assert_eq!(&odd.pixels[48..56], &eight);
        assert_eq!(odd_record.x, 12); // (16 / 8) * 6
        assert_eq!(odd_record.y, 6); // (4 / 4) * 6
    }

    #[test]
    fn page_dispatch_matches_bpp_tags() {
        let mut dxt1 = W13A01TilePage {
            bpp: W13A01_BPP_DXT1,
            tile_size: 8,
            width: 64,
            height: 32,
            ..Default::default()
        };
        let mut palette = W13A01TilePage {
            bpp: W13A01_BPP_PALETTE,
            tile_size: 8,
            width: 64,
            height: 32,
            ..Default::default()
        };
        assert_eq!(w13a01_page_size(&dxt1), 0x400);
        assert_eq!(w13a01_page_size(&palette), 0x800);
        assert!(!w13a01_page_is_full(&dxt1));
        dxt1.cursor_y = 64;
        assert!(w13a01_page_is_full(&dxt1));
        assert!(!w13a01_page_is_full(&palette));
        palette.cursor_y = 32;
        assert!(w13a01_page_is_full(&palette));
    }

    #[test]
    fn tileset_word_and_tile_size_accessors() {
        // 0x0065c910 / 0x0065cb00 / 0x0065c920
        let mut set = palette_tileset();
        set.pages.push(W13A01TilePage::default());
        set.pages.push(W13A01TilePage::default());
        set.tile_size = 0x20;
        assert_eq!(w13a01_tileset_page_count(&set), 2);
        assert_eq!(w13a01_tileset_tile_size(&set), 0x20);
        assert_eq!(w13a01_tileset_scaled_tile_size(&set, 4), 8);
        assert_eq!(w13a01_tileset_scaled_tile_size(&set, 0), 0x20);
        assert_eq!(w13a01_tileset_scaled_tile_size(&set, 5), 6);
    }

    #[test]
    fn tile_presence_reads_the_presence_map() {
        // 0x0065cf40
        let mut set = palette_tileset();
        set.present = vec![0, 1, 0, 7];
        assert_eq!(w13a01_tile_presence(&set, 0), Some(0));
        assert_eq!(w13a01_tile_presence(&set, 1), Some(1));
        assert_eq!(w13a01_tile_presence(&set, 3), Some(7));
        assert_eq!(w13a01_tile_presence(&set, 4), None);
        assert_eq!(w13a01_tile_presence(&set, u16::MAX), None);
    }

    #[test]
    fn tile_page_at_returns_the_owning_page() {
        // 0x0065caa0
        let mut set = palette_tileset();
        set.pages = vec![
            W13A01TilePage {
                kind: 4,
                ..Default::default()
            },
            W13A01TilePage {
                kind: 5,
                ..Default::default()
            },
        ];
        assert_eq!(w13a01_tile_page_at(&set, 0).map(|p| p.kind), Some(4));
        assert_eq!(w13a01_tile_page_at(&set, 1).map(|p| p.kind), Some(5));
        assert_eq!(w13a01_tile_page_at(&set, 2), None);
        assert_eq!(w13a01_tile_page_at(&set, u16::MAX), None);
    }

    #[test]
    fn configure_resizes_records_and_presence_with_placeholders() {
        // 0x0065cb10
        let mut set = palette_tileset();
        set.records = vec![W13A01TileRecord {
            id: 1,
            page: 2,
            frame: 3,
            x: 4,
            y: 5,
        }];
        set.present = vec![9];
        w13a01_configure_tile_set(&mut set, 3, 16, 0xabcd, true);
        assert_eq!(set.tile_size, 16);
        assert_eq!(set.owner_tag, 0xabcd);
        assert!(set.use_dxt1);
        assert_eq!(set.records.len(), 3);
        assert_eq!(set.records[0].id, 1);
        assert_eq!(set.records[1], W13A01TileRecord::PLACEHOLDER);
        assert_eq!(set.records[2], W13A01TileRecord::PLACEHOLDER);
        assert_eq!(set.present, vec![9, 0, 0]);

        // Shrinking keeps the prefix and drops the tail.
        w13a01_configure_tile_set(&mut set, 1, 8, 1, false);
        assert_eq!(set.records.len(), 1);
        assert_eq!(set.records[0].id, 1);
        assert_eq!(set.present, vec![9]);
        assert!(!set.use_dxt1);
    }

    #[test]
    fn tile_uv_lookup_rejects_wrong_id_and_missing_page() {
        // 0x0065c960
        let mut set = palette_tileset();
        set.tile_size = 16;
        set.pages = vec![W13A01TilePage {
            width: 64,
            height: 32,
            ..Default::default()
        }];
        set.records = vec![W13A01TileRecord::default(); 6];
        set.records[0].id = 0xdead;
        set.records[5] = W13A01TileRecord {
            id: 5,
            page: 0,
            frame: 9,
            x: 16,
            y: 8,
        };
        let uv = w13a01_tile_uv(&set, 5).expect("known tile");
        assert_eq!(uv.page, 0);
        assert_eq!(uv.frame, 9);
        assert_eq!(uv.u0, 0.25);
        assert_eq!(uv.v0, 0.25);
        assert_eq!(uv.u1, 0.25);
        assert_eq!(uv.v1, 0.5);

        // Requesting id 0 reads record 0, whose stored id is 5 -> reject.
        assert_eq!(w13a01_tile_uv(&set, 0), None);
        assert_eq!(w13a01_tile_uv(&set, 1), None);
        let mut missing_page = set.clone();
        missing_page.records[5].page = 7;
        assert_eq!(w13a01_tile_uv(&missing_page, 5), None);
        set.pages[0].width = 0;
        assert_eq!(w13a01_tile_uv(&set, 5), None);
    }

    #[test]
    fn new_tile_page_uses_tileset_fields_and_zero_fills_pixels() {
        // 0x0065c410
        let mut set = W13A01Tileset {
            pages: Vec::new(),
            owner_tag: 0x77,
            tile_size: 8,
            use_dxt1: false,
            width: 32,
            height: 32,
            ..Default::default()
        };
        let index = w13a01_new_tile_page(&mut set, 0);
        assert_eq!(index, 0);
        assert_eq!(set.pages.len(), 1);
        let page = &set.pages[0];
        assert_eq!(page.index, 0);
        assert_eq!(page.kind, 0);
        assert_eq!(page.bpp, W13A01_BPP_PALETTE);
        assert_eq!(page.tile_size, 8);
        assert_eq!(page.width, 32);
        assert_eq!(page.height, 32);
        assert_eq!(page.owner_tag, 0x77);
        assert_eq!(page.pixels.len(), 32 * 32);
        assert!(page.pixels.iter().all(|&b| b == 0));
        assert_eq!(page.cursor_x, 0);
        assert_eq!(page.cursor_y, 0);
        assert_eq!(page.refs, 0);
        assert!(!page.sealed);

        set.use_dxt1 = true;
        let index = w13a01_new_tile_page(&mut set, W13A01_KIND_DXT1);
        assert_eq!(index, 1);
        let page = &set.pages[1];
        assert_eq!(page.index, 1);
        assert_eq!(page.kind, W13A01_KIND_DXT1);
        assert_eq!(page.bpp, W13A01_BPP_DXT1);
        assert_eq!(page.pixels.len(), (32 * 32) / 2);
    }

    #[test]
    fn find_tile_page_skips_full_and_sealed_pages() {
        // 0x0065c1f0 selection loop
        let mut set = palette_tileset();
        set.pages = vec![
            W13A01TilePage {
                kind: 0,
                height: 32,
                cursor_y: 32, // full
                ..Default::default()
            },
            W13A01TilePage {
                kind: 0,
                height: 32,
                sealed: true, // sealed
                ..Default::default()
            },
            W13A01TilePage {
                kind: 1,
                height: 32,
                ..Default::default()
            },
            W13A01TilePage {
                kind: 0,
                height: 32,
                cursor_y: 8,
                ..Default::default()
            },
        ];
        assert_eq!(w13a01_find_tile_page(&set, 0), Some(3));
        assert_eq!(w13a01_find_tile_page(&set, 1), Some(2));
        assert_eq!(w13a01_find_tile_page(&set, 2), None);
    }

    #[test]
    fn place_tile_creates_page_then_reuses_it_and_writes_page_slot() {
        // 0x0065c1f0 + 0x0065c410
        let mut set = W13A01Tileset {
            tile_size: 2,
            width: 4,
            height: 4,
            use_dxt1: false,
            ..Default::default()
        };
        // Palette blit needs 4 bytes per tile; page is 4x4, so four 2x2
        // tiles fit per row and two rows per page: 8 tiles total.
        let source = [1u8, 2, 3, 4];
        let mut record = W13A01TileRecord {
            id: 1,
            ..Default::default()
        };
        assert!(w13a01_place_tile(&mut set, 0, &mut record, &source));
        assert_eq!(set.pages.len(), 1);
        assert_eq!(record.page, 0);
        assert_eq!(set.pages[0].refs, 1);
        assert_eq!(set.pages[0].pixels[0], 1);

        let mut second = W13A01TileRecord {
            id: 2,
            ..Default::default()
        };
        assert!(w13a01_place_tile(&mut set, 0, &mut second, &source));
        assert_eq!(set.pages.len(), 1);
        assert_eq!(second.page, 0);
        assert_eq!(set.pages[0].refs, 2);

        // A different kind forces a new page and stores it in the record's
        // primary slot (kind != 2).
        let mut third = W13A01TileRecord {
            id: 3,
            ..Default::default()
        };
        assert!(w13a01_place_tile(&mut set, 1, &mut third, &source));
        assert_eq!(set.pages.len(), 2);
        assert_eq!(third.page, 1);
        assert_eq!(set.pages[1].kind, 1);

        // Kind 2 writes the page index into the frame slot.
        let mut fourth = W13A01TileRecord {
            id: 4,
            ..Default::default()
        };
        assert!(w13a01_place_tile(
            &mut set,
            W13A01_KIND_DXT1,
            &mut fourth,
            &source
        ));
        assert_eq!(set.pages.len(), 3);
        assert_eq!(fourth.frame, 2);
        assert_eq!(fourth.page, 0);
    }

    #[test]
    fn place_tile_returns_false_when_the_new_page_is_already_full() {
        // 0x0065c1f0 miss path: zero-sized palette page is full at y=0.
        let mut set = W13A01Tileset {
            tile_size: 2,
            width: 4,
            height: 0,
            use_dxt1: false,
            ..Default::default()
        };
        let mut record = W13A01TileRecord::default();
        assert!(!w13a01_place_tile(&mut set, 0, &mut record, &[1, 2, 3, 4]));
        assert_eq!(set.pages.len(), 1);
        // The record slot is untouched on failure.
        assert_eq!(record.page, 0);
    }

    #[test]
    fn release_owned_resources_drops_first_two_and_conditional_third() {
        // 0x0065c0b0
        let mut resources = W13A01OwnedResources {
            first: Some(vec![1]),
            second: Some(vec![2]),
            third: Some(vec![3]),
            third_state: 0,
        };
        w13a01_release_owned_resources(&mut resources);
        assert_eq!(resources.first, None);
        assert_eq!(resources.second, None);
        assert_eq!(resources.third, None);
        assert_eq!(resources.third_state, 0);

        let mut negative = W13A01OwnedResources {
            first: None,
            second: None,
            third: Some(vec![3]),
            third_state: -1,
        };
        w13a01_release_owned_resources(&mut negative);
        assert_eq!(negative.third, Some(vec![3]));
    }

    #[test]
    fn deleting_destructors_report_the_shipped_free_sizes() {
        // 0x0065c140
        let mut manager = W13A01SharedPtrManager::default();
        assert_eq!(
            w13a01_shared_ptr_manager_deleting_dtor(&mut manager, 0),
            None
        );
        assert_eq!(manager.vtable, W13A01_SHARED_PTR_MANAGER_VTABLE);
        assert_eq!(
            w13a01_shared_ptr_manager_deleting_dtor(&mut manager, 1),
            Some(0x50)
        );
        assert_eq!(
            w13a01_shared_ptr_manager_deleting_dtor(&mut manager, 0xfe),
            None
        );

        // 0x0065c170
        let mut page = W13A01TilePageDeletion {
            surface: Some(vec![0; 16]),
            surface_state: 0,
            base_live: true,
            ..Default::default()
        };
        assert_eq!(w13a01_tile_page_deleting_dtor(&mut page, 0), None);
        assert_eq!(page.vtable, W13A01_TILE_PAGE_VTABLE);
        assert_eq!(page.surface, None);
        assert!(!page.base_live);
        assert_eq!(w13a01_tile_page_deleting_dtor(&mut page, 1), Some(0x44));

        let mut retained = W13A01TilePageDeletion {
            surface: Some(vec![7]),
            surface_state: -1,
            base_live: true,
            ..Default::default()
        };
        assert_eq!(w13a01_tile_page_deleting_dtor(&mut retained, 0), None);
        assert_eq!(retained.surface, Some(vec![7]));
        assert!(!retained.base_live);
    }
}

// ===========================================================================
// Merged from src/wave14/agent03.rs
// ===========================================================================
// Wave 14 translation batch 03.
//
// Window `00674000..00675fff`: the `WritableBufferTextureFactory`
// surface/render-queue helpers, the `blz` red-black map routines they use,
// and the TinyXML2 `XMLPrinter`/`XMLVisitor` destructors.
//
// ## Recovered layouts (patch 1.23.10)
//
// Surface object (`0x006744c0` constructor):
//
// | offset | type | meaning |
// |---|---|---|
// | `+0x00` | `u32` | width |
// | `+0x04` | `u32` | height |
// | `+0x08` | `u32` | format/aux field stored by the constructor |
// | `+0x0c` | `0x10` bytes | embedded context constructed by `FUN_00a892d0` |
// | `+0x1c` | `u8` | surface-created flag |
// | `+0x20` | `u32` | render mode (`0x006746e0`) |
// | `+0x24` | list | rect queue header: next, prev, count |
// | `+0x30` | `std::vector<u32>` | pixel buffer (begin, size, capacity) |
// | `+0x3c` | `u8` | dirty flag |
//
// A queue node is 24 bytes: `next`, `prev`, then a four-dword rect
// `{x, y, width, height}`. The map value embedded in a tree node starts at
// node `+0x18`; its `+0x00` dword is a tag, `+0x08` is a surface pointer and
// `+0x0c` is a mark byte.
//
// The map is the old MSVC/Dinkumware red-black tree layout:
// node `+0x00` parent, `+0x04` left, `+0x08` right, `+0x0c` colour
// (`0` red, non-zero black), `+0x10` key.first, `+0x14` key.second. Keys are
// ordered by `(key.second, key.first)`. The header is the map object
// `+0x14`; header `+0x00` is the root, `+0x04` the leftmost node and `+0x08`
// the rightmost node.
//
// The TinyXML2 printer owns two `std::string`-like blocks at `+0x08`
// (inline storage `+0x0c`) and `+0xd0` (inline storage `+0xd4`). Its visitor
// base vtable is `0x00d3e2c4`, the printer vtable `0x00d3e2ec`; the deleting
// destructor frees `0xf0` bytes. `WritableBufferTextureFactory` uses the
// vtables `0x00d3e268` / `0x00d3e284` and the `IBrowserTextureFactoryV1`
// vtable is `0x00d3e24c`.
//
// ## Skipped rows
//
// | rows | reason |
// |---|---|
// | `FUN_00674000`, `FUN_00674020`, `FUN_00674430`, `FUN_00674440`, `FUN_00674ab2`, `FUN_006759d0` | thin forwarding wrappers around opaque callees; the wrapper has no recoverable value-level result |
// | `FUN_00674050`, `FUN_0067440a`, `FUN_00675020`, `FUN_006759a9`, `FUN_00675e8f` | exception-unwind funclets; both visible paths end in non-returning cleanup |
// | `FUN_00674236`, `FUN_006750ab` | mid-function fragments over `unaff_*`/`in_*` registers with no recoverable arguments |
// | `FUN_0067424f`, `FUN_0067426b` | overlapping instructions, an unrecovered jump table and `WARNING: Control flow encountered bad instruction data` |
// | `FUN_00674be0` | map `operator[]` body: allocates a 0x40-byte surface and a 0x28-byte node, then delegates the red-black link/rebalance to the opaque `FUN_00597290`; only the search half (`006752f0`) is translated |
// | `FUN_00674d50` | map erase destroys the mapped surface but delegates node unlink/rebalance to the opaque `FUN_00596fa0`; the whole operation is not recoverable without inventing the reassembly |
// | `FUN_00674e60` | one-byte non-returning runtime trap |
// | `FUN_00675150` | tree-copy body mixes the subtree clone with allocator/timestamp writes and non-returning cleanup |
// | `FUN_00675530`, `FUN_00675610`, `FUN_00675780`, `FUN_00675890`, `FUN_00675900`, `FUN_006759f0`, `FUN_00675af0`, `FUN_00675ee0`, `FUN_00675fe0`, `FUN_00675ff4` | `std::string` construction/exception fragments, caller-frame stack slots and non-returning cleanup; no standalone pure result |
// | `FUN_006755e0` | three opaque sequencing calls; no recoverable value |
// | `FUN_00675bd8`, `FUN_00675c47`, `FUN_00675d04`, `FUN_00675d5e`, `FUN_00675e05`, `FUN_00675e44` | vector-growth/append blocks that share `unaff_EBX`/`unaff_EBP` registers and caller-owned temporaries |
// | `thunk_FUN_00674236` | two-byte thunk |
//
// Rows not listed in the tables above are translated in this file.

// ===========================================================================
// Small string/state helpers
// ===========================================================================

/// `0x00674460`: return the first index `>= from` whose byte equals
/// `wanted`, or `None` when `from` is at/after the end.
///
/// The shipped body walks the `blz::string` buffer (`this[0]` = data,
/// `this[1]` = length) and returns `0xffffffff` on failure.
pub fn w14a03_find_char(buffer: &[u8], from: usize, wanted: u8) -> Option<usize> {
    let mut index = from;
    loop {
        if index >= buffer.len() {
            return None;
        }
        if buffer[index] == wanted {
            return Some(index);
        }
        index += 1;
    }
}

/// `0x00674e40`: store two dwords at object offsets `+0x50` and `+0x54`.
///
/// The translated helper takes the dword view of the object (22 dwords cover
/// `0x00..0x58`). It returns `false` instead of reading past a short slice.
pub fn w14a03_write_fields_50_54(state: &mut [u32], first: u32, second: u32) -> bool {
    if state.len() < 22 {
        return false;
    }
    state[0x50 / 4] = first;
    state[0x54 / 4] = second;
    true
}

/// `0x00675770`: read the byte at object offset `+0x4d`.
///
/// The shipped code has no bounds check; this version returns `None` for a
/// slice that does not contain the field.
pub fn w14a03_byte_4d(state: &[u8]) -> Option<u8> {
    state.get(0x4d).copied()
}

// ===========================================================================
// WritableBufferTextureFactory surface
// ===========================================================================

/// A rect record stored in the surface queue (`{x, y, width, height}`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14a03Rect {
    /// `+0x00`: left edge.
    pub x: u32,
    /// `+0x04`: top edge.
    pub y: u32,
    /// `+0x08`: width; a zero width is skipped by the flush.
    pub width: u32,
    /// `+0x0c`: height; a zero height is skipped by the flush.
    pub height: u32,
}

/// The surface object created by `0x006744c0`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W14a03Surface {
    /// `+0x00`: width in pixels.
    pub width: u32,
    /// `+0x04`: height in pixels.
    pub height: u32,
    /// `+0x08`: format/aux value handed to the blitter.
    pub field_08: u32,
    /// `+0x1c`: set once the opaque surface-creation call succeeded.
    pub initialized: bool,
    /// `+0x20`: render mode; `1` selects source alpha in the blitter calls.
    pub mode: u32,
    /// `+0x24`: queued rects in list order.
    pub rects: Vec<W14a03Rect>,
    /// `+0x30`: zero-filled `width * height` dword buffer.
    pub pixels: Vec<u32>,
    /// `+0x3c`: dirty flag.
    pub dirty: bool,
}

/// The six-dword blit-source descriptor written by `0x00674670`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14a03BlitDescriptor {
    /// `+0x00`: destination/source X (always zero in the recovered path).
    pub x: u32,
    /// `+0x04`: destination/source Y (always zero in the recovered path).
    pub y: u32,
    /// `+0x08`: surface width.
    pub width: u32,
    /// `+0x0c`: surface height.
    pub height: u32,
    /// `+0x10`: whether the `+0x30` pixel vector has a buffer.
    pub pixels_available: bool,
    /// `+0x14`: row stride, copied from the width.
    pub stride: u32,
}

/// One rect presented by `0x006745e0`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct W14a03PresentCall {
    /// Rect passed as the second argument.
    pub rect: W14a03Rect,
    /// Blend mode constant pushed by the shipped code (`0x16`).
    pub blend_mode: u32,
    /// Colour vector loaded from `DAT_00f3ca88` (four `1.0f` values).
    pub color: [f32; 4],
    /// Trailing flag argument (zero).
    pub flags: u32,
}

/// One full-surface clear/draw request issued by `0x006746e0`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14a03ClearCall {
    /// Surface `+0x08` value forwarded to the draw call.
    pub field_08: u32,
    /// `mode == 1`.
    pub use_source_alpha: bool,
    /// Trailing flag argument (zero).
    pub flags: u32,
}

/// One queued blit produced by the flush at `0x00674730`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14a03BlitCall {
    /// Rect X.
    pub x: u32,
    /// Rect Y.
    pub y: u32,
    /// Rect width (`> 0`).
    pub width: u32,
    /// Rect height (`> 0`).
    pub height: u32,
    /// Dword index into the pixel vector: `y * surface.width + x`.
    pub source_index: u32,
    /// Destination stride in pixels (surface width).
    pub pitch: u32,
    /// Surface `+0x08` value.
    pub field_08: u32,
    /// `mode == 1`.
    pub use_source_alpha: bool,
    /// Trailing flag argument (zero).
    pub flags: u32,
}

/// Result of the flush decision at `0x00674730`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W14a03FlushPlan {
    /// The shipped body calls the surface constructor at `+0x0c` first.
    pub create_surface: bool,
    /// State of the `+0x1c` flag after the call.
    pub initialized: bool,
    /// Blits in queue order (zero-sized rects omitted).
    pub blits: Vec<W14a03BlitCall>,
    /// The `+0x3c` dirty byte is cleared on every path.
    pub cleared_dirty: bool,
}

/// `0x006744c0`: construct the surface header and resize the pixel vector to
/// `width * height` zeroed dwords.
///
/// The embedded context at `+0x0c` is constructed by the opaque
/// `FUN_00a892d0`; that object is not modelled. The size multiply is a
/// 32-bit wrapping multiply, matching the shipped `imul`.
pub fn w14a03_surface_new(width: u32, height: u32, field_08: u32, mode: u32) -> W14a03Surface {
    let count = width.wrapping_mul(height) as usize;
    W14a03Surface {
        width,
        height,
        field_08,
        initialized: false,
        mode,
        rects: Vec::new(),
        pixels: vec![0; count],
        dirty: false,
    }
}

/// `0x00674630`: append a four-dword rect to the surface queue.
///
/// The shipped helper allocates a 24-byte node and links it at the tail of
/// the `+0x24` header; returns the zero-based position of the new record.
pub fn w14a03_surface_push_rect(surface: &mut W14a03Surface, rect: W14a03Rect) -> usize {
    let index = surface.rects.len();
    surface.rects.push(rect);
    index
}

/// `0x00674670`: build the six-dword blit descriptor from the surface and
/// set the `+0x3c` dirty flag.
///
/// The shipped code stores the raw `+0x30` buffer pointer in slot `+0x10`;
/// this version records only whether that vector has a buffer.
pub fn w14a03_blit_descriptor(surface: &mut W14a03Surface) -> W14a03BlitDescriptor {
    surface.dirty = true;
    W14a03BlitDescriptor {
        x: 0,
        y: 0,
        width: surface.width,
        height: surface.height,
        pixels_available: !surface.pixels.is_empty(),
        stride: surface.width,
    }
}

/// `0x006745e0`: present one rect when the surface-created flag is set.
///
/// The recovered call uses blend mode `0x16`, a colour vector of four `1.0f`
/// values read from `DAT_00f3ca88`, and a trailing zero. The opaque context
/// lock at `+0x0c` is not modelled.
pub fn w14a03_present_call(surface: &W14a03Surface, rect: W14a03Rect) -> Option<W14a03PresentCall> {
    if !surface.initialized {
        return None;
    }
    Some(W14a03PresentCall {
        rect,
        blend_mode: 0x16,
        color: [1.0; 4],
        flags: 0,
    })
}

/// `0x006746e0`: store the render mode and, when the opaque readiness check
/// succeeds, issue the full-surface clear/draw call.
///
/// The caller supplies the result of `FUN_0067acc0` as `render_ready`.
pub fn w14a03_set_surface_mode(
    surface: &mut W14a03Surface,
    mode: u32,
    render_ready: bool,
) -> Option<W14a03ClearCall> {
    surface.mode = mode;
    if !render_ready {
        return None;
    }
    Some(W14a03ClearCall {
        field_08: surface.field_08,
        use_source_alpha: mode == 1,
        flags: 0,
    })
}

/// `0x00674730`: flush the queued rects.
///
/// An empty queue only clears the dirty byte. Otherwise the shipped body
/// creates the surface when `+0x1c` is clear, checks readiness through
/// `FUN_0067acc0`, emits one blit per non-empty rect and finally clears the
/// queue and the dirty byte. `render_ready` models both opaque readiness
/// results; a failed readiness leaves the queue intact.
pub fn w14a03_flush_surface(surface: &mut W14a03Surface, render_ready: bool) -> W14a03FlushPlan {
    let mut plan = W14a03FlushPlan::default();
    if surface.rects.is_empty() {
        surface.dirty = false;
        plan.cleared_dirty = true;
        return plan;
    }
    if !surface.initialized {
        plan.create_surface = true;
    }
    if !render_ready {
        surface.dirty = false;
        plan.cleared_dirty = true;
        return plan;
    }
    surface.initialized = true;
    let width = surface.width;
    for rect in &surface.rects {
        if rect.width == 0 || rect.height == 0 {
            continue;
        }
        let source_index = rect.y.wrapping_mul(width).wrapping_add(rect.x);
        plan.blits.push(W14a03BlitCall {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
            source_index,
            pitch: width,
            field_08: surface.field_08,
            use_source_alpha: surface.mode == 1,
            flags: 0,
        });
    }
    surface.rects.clear();
    surface.dirty = false;
    plan.initialized = true;
    plan.cleared_dirty = true;
    plan
}

// ===========================================================================
// blz map tree helpers
// ===========================================================================

/// One key of the map; comparison is `(second, first)`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14a03MapKey {
    /// Node `+0x10`.
    pub first: u32,
    /// Node `+0x14`; the primary comparison field.
    pub second: u32,
}

/// One red-black tree node.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W14a03MapNode {
    /// `+0x00`: parent (`None` for the root).
    pub parent: Option<usize>,
    /// `+0x04`: left child.
    pub left: Option<usize>,
    /// `+0x08`: right child.
    pub right: Option<usize>,
    /// `+0x0c`: colour; `true` is black.
    pub is_black: bool,
    /// `+0x10..+0x18`: key.
    pub key: W14a03MapKey,
    /// `+0x18`: first dword of the mapped value.
    pub value_tag: u32,
    /// `+0x20`: mapped surface index.
    pub surface: Option<usize>,
    /// `+0x24`: value mark byte.
    pub marked: bool,
}

/// The map object with the header at `+0x14`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W14a03MapTree {
    /// Nodes addressed by index (the shipped code uses pointers).
    pub nodes: Vec<W14a03MapNode>,
    /// Header `+0x00`: tree root.
    pub root: Option<usize>,
    /// Header `+0x04`: leftmost node.
    pub leftmost: Option<usize>,
    /// Header `+0x08`: rightmost node.
    pub rightmost: Option<usize>,
    /// Map object `+0x5c`: insertion/update counter.
    pub counter_5c: u32,
    /// Map object `+0x58`: stored mode used by the bulk update.
    pub field_58: u32,
}

/// Which link the insertion helper at `0x006752f0` selected.
///
/// The shipped helper returns a pointer to the link to fill, or null. The
/// caller at `00674be0` interprets the null link as "key already present"
/// and updates the returned node in place.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum W14a03InsertLink {
    /// The shipped helper returned null: the node already carries the key.
    #[default]
    Existing,
    /// Attach as the node's right child.
    Right,
    /// Attach at `header + 0x04` (the new leftmost node).
    HeaderLeft,
}

/// Result of the insertion search at `0x006752f0`.
///
/// `node == None` means the header sentinel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14a03InsertHint {
    /// Node selected by the search.
    pub node: Option<usize>,
    /// Link to fill.
    pub link: W14a03InsertLink,
}

/// One entry selected by the bulk mode update at `0x006751d0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W14a03ModeTarget {
    /// Node index in `W14a03MapTree::nodes`.
    pub entry: usize,
    /// Mapped surface index.
    pub surface: usize,
}

/// `0x006753f0`: first node whose key is not less than `key`.
///
/// This is the shipped `_Lbound` walk; a missing result is the header
/// sentinel, represented here as `None`.
pub fn w14a03_tree_lower_bound(tree: &W14a03MapTree, key: W14a03MapKey) -> Option<usize> {
    let mut current = tree.root?;
    let mut candidate = None;
    loop {
        let node = &tree.nodes[current];
        if w14a03_key_less(&node.key, &key) {
            match node.right {
                Some(right) => current = right,
                None => break,
            }
        } else {
            candidate = Some(current);
            match node.left {
                Some(left) => current = left,
                None => break,
            }
        }
    }
    candidate
}

/// `0x00674f00`: successor step used while marking every map entry.
///
/// The shipped pointer version is the standard in-order successor of the old
/// MSVC tree iterator; `None` stands for the header sentinel.
pub fn w14a03_tree_successor(tree: &W14a03MapTree, node: usize) -> Option<usize> {
    if let Some(right) = tree.nodes[node].right {
        let mut current = right;
        while let Some(left) = tree.nodes[current].left {
            current = left;
        }
        return Some(current);
    }
    let mut current = node;
    let mut parent = tree.nodes[current].parent?;
    while tree.nodes[current].parent == Some(parent) && tree.nodes[parent].right == Some(current) {
        current = parent;
        parent = tree.nodes[current].parent?;
    }
    Some(parent)
}

/// `0x00674f00`: set the mark byte at value `+0x0c` (node `+0x24`) on every
/// entry, walking in order from the header's leftmost node. Returns the
/// number of entries marked.
pub fn w14a03_tree_mark_all(tree: &mut W14a03MapTree) -> usize {
    let mut current = tree.leftmost;
    let mut count = 0;
    while let Some(index) = current {
        tree.nodes[index].marked = true;
        count += 1;
        current = w14a03_tree_successor(tree, index);
    }
    count
}

/// `0x00675280`: clone one subtree into a fresh arena.
///
/// The shipped `_Copy_node` helper allocates the new node, copies the value
/// and colour, then recursively copies the left and right children. The
/// returned index is the clone root.
pub fn w14a03_tree_clone_subtree(
    tree: &mut W14a03MapTree,
    source: Option<usize>,
    parent: Option<usize>,
) -> Option<usize> {
    let source = source?;
    let original = tree.nodes[source].clone();
    let index = tree.nodes.len();
    tree.nodes.push(W14a03MapNode {
        parent,
        left: None,
        right: None,
        is_black: original.is_black,
        key: original.key,
        value_tag: original.value_tag,
        surface: original.surface,
        marked: original.marked,
    });
    let left = w14a03_tree_clone_subtree(tree, original.left, Some(index));
    let right = w14a03_tree_clone_subtree(tree, original.right, Some(index));
    tree.nodes[index].left = left;
    tree.nodes[index].right = right;
    Some(index)
}

/// `0x006752f0`: locate the insertion link for `key`.
///
/// The shipped `_Find` walk records the last visited node and whether the
/// final step went left. A right step either selects the node's right link
/// (key greater than the node) or reports the existing node (key equal to
/// it). A left step off the leftmost node selects the header's left link;
/// otherwise the predecessor is returned. The original red/left cycle check
/// (`node.parent.parent == node`) can only fire for a malformed tree and is
/// not modelled.
pub fn w14a03_tree_insert_hint(tree: &W14a03MapTree, key: W14a03MapKey) -> W14a03InsertHint {
    let Some(root) = tree.root else {
        return W14a03InsertHint {
            node: None,
            link: W14a03InsertLink::HeaderLeft,
        };
    };
    let mut current = root;
    let mut last;
    let mut went_left;
    loop {
        last = current;
        let node = &tree.nodes[current];
        if w14a03_key_less(&key, &node.key) {
            went_left = true;
            match node.left {
                Some(left) => current = left,
                None => break,
            }
        } else {
            went_left = false;
            match node.right {
                Some(right) => current = right,
                None => break,
            }
        }
    }
    if !went_left {
        let node = &tree.nodes[last];
        return W14a03InsertHint {
            node: Some(last),
            link: if w14a03_key_less(&node.key, &key) {
                W14a03InsertLink::Right
            } else {
                W14a03InsertLink::Existing
            },
        };
    }
    if tree.leftmost == Some(last) {
        return W14a03InsertHint {
            node: None,
            link: W14a03InsertLink::HeaderLeft,
        };
    }
    let predecessor = w14a03_tree_predecessor(tree, last);
    match predecessor {
        Some(index) if w14a03_key_less(&tree.nodes[index].key, &key) => W14a03InsertHint {
            node: None,
            link: W14a03InsertLink::HeaderLeft,
        },
        Some(index) => W14a03InsertHint {
            node: Some(index),
            link: W14a03InsertLink::Existing,
        },
        None => W14a03InsertHint {
            node: None,
            link: W14a03InsertLink::HeaderLeft,
        },
    }
}

/// Predecessor step used by `0x006752f0`'s insertion search.
fn w14a03_tree_predecessor(tree: &W14a03MapTree, node: usize) -> Option<usize> {
    if let Some(left) = tree.nodes[node].left {
        let mut current = left;
        while let Some(right) = tree.nodes[current].right {
            current = right;
        }
        return Some(current);
    }
    let mut current = node;
    let mut parent = tree.nodes[current].parent?;
    while tree.nodes[parent].left == Some(current) {
        current = parent;
        parent = tree.nodes[current].parent?;
    }
    Some(parent)
}

/// `0x00674f50`: bump the map counter, take the lower-bound entry and append
/// `rect` to its mapped surface queue. Returns whether a surface was reached.
///
/// The shipped body does not check key equality: the lower-bound node is used
/// directly. A null mapped surface is skipped here instead of dereferenced.
pub fn w14a03_tree_append_rect(
    tree: &mut W14a03MapTree,
    key: W14a03MapKey,
    rect: W14a03Rect,
    surfaces: &mut [W14a03Surface],
) -> bool {
    tree.counter_5c = tree.counter_5c.wrapping_add(1);
    let Some(entry) = w14a03_tree_lower_bound(tree, key) else {
        return false;
    };
    let Some(surface) = tree.nodes[entry].surface else {
        return false;
    };
    if let Some(target) = surfaces.get_mut(surface) {
        w14a03_surface_push_rect(target, rect);
        true
    } else {
        false
    }
}

/// `0x00674fc0`: return the lower-bound entry's pixel buffer.
///
/// The shipped body calls `0x00674670` on the mapped surface and returns
/// descriptor slot `+0x10` (the buffer pointer), or zero when the entry or
/// surface is missing.
pub fn w14a03_tree_entry_pixels<'a>(
    tree: &W14a03MapTree,
    key: W14a03MapKey,
    surfaces: &'a [W14a03Surface],
) -> Option<&'a [u32]> {
    let entry = w14a03_tree_lower_bound(tree, key)?;
    let surface = tree.nodes[entry].surface?;
    let pixels = &surfaces.get(surface)?.pixels;
    if pixels.is_empty() {
        None
    } else {
        Some(pixels)
    }
}

/// `0x006751d0`: store `mode` at map `+0x58` and enumerate every entry's
/// mapped surface in tree order.
///
/// Each returned target is the call site of `0x006746e0` in the shipped loop;
/// the caller applies `w14a03_set_surface_mode` to it.
pub fn w14a03_tree_set_mode_all(tree: &mut W14a03MapTree, mode: u32) -> Vec<W14a03ModeTarget> {
    tree.field_58 = mode;
    let mut targets = Vec::new();
    let mut current = tree.leftmost;
    while let Some(index) = current {
        if let Some(surface) = tree.nodes[index].surface {
            targets.push(W14a03ModeTarget {
                entry: index,
                surface,
            });
        }
        current = w14a03_tree_successor(tree, index);
    }
    targets
}

/// `0x00675230`: return the lower-bound entry's mapped surface index.
///
/// The shipped body calls the flush at `0x00674730` on that surface when the
/// entry exists; the caller performs the flush with
/// [`w14a03_flush_surface`].
pub fn w14a03_tree_flush_target(tree: &W14a03MapTree, key: W14a03MapKey) -> Option<usize> {
    let entry = w14a03_tree_lower_bound(tree, key)?;
    tree.nodes[entry].surface
}

/// Compare two map keys in the shipped `(second, first)` order.
fn w14a03_key_less(left: &W14a03MapKey, right: &W14a03MapKey) -> bool {
    (left.second, left.first) < (right.second, right.first)
}

// ===========================================================================
// TinyXML2 and factory vtables / destructors
// ===========================================================================

/// `0x00674960`: install the `IBrowserTextureFactoryV1` vtable.
pub fn w14a03_install_browser_texture_factory_vtable(vtable: &mut u32) {
    *vtable = 0x00d3_e24c;
}

/// `0x00674ac0`: scalar deleting destructor of the object built by
/// `0x00674960`.
///
/// The body stores the `0x00d3e24c` vtable and calls `operator delete(this, 4)`
/// when bit 0 of the flag byte is set.
pub fn w14a03_browser_factory_delete_plan(flags: u8) -> W14a03DeletePlan {
    W14a03DeletePlan {
        final_vtable: 0x00d3_e24c,
        delete_self: flags & 1 != 0,
        delete_size: 4,
    }
}

/// `0x00675490`: install the `tinyxml2::XMLVisitor` vtable.
pub fn w14a03_install_xml_visitor_vtable(vtable: &mut u32) {
    *vtable = 0x00d3_e2c4;
}

/// `0x00675500`: scalar deleting destructor of `tinyxml2::XMLVisitor`.
pub fn w14a03_xml_visitor_delete_plan(vtable: &mut u32, flags: u8) -> W14a03DeletePlan {
    *vtable = 0x00d3_e2c4;
    W14a03DeletePlan {
        final_vtable: *vtable,
        delete_self: flags & 1 != 0,
        delete_size: 4,
    }
}

/// Result of a scalar deleting destructor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W14a03DeletePlan {
    /// Vtable left in `+0x00`.
    pub final_vtable: u32,
    /// Whether `operator delete(this, delete_size)` runs.
    pub delete_self: bool,
    /// Allocation size passed to `operator delete`.
    pub delete_size: usize,
}

/// One of the two `std::string`-like blocks owned by the XML printer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum W14a03XmlBlock {
    /// Buffer points at the inline storage, so the destructor does not free.
    #[default]
    Inline,
    /// Buffer points at a heap allocation released by the destructor.
    Heap(Vec<u8>),
}

/// The two owned blocks of `tinyxml2::XMLPrinter`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W14a03XmlPrinterState {
    /// `+0x00`: vtable.
    pub vtable: u32,
    /// `+0x08`: block whose inline storage is `+0x0c`.
    pub block_08: W14a03XmlBlock,
    /// `+0xd0`: block whose inline storage is `+0xd4`.
    pub block_d0: W14a03XmlBlock,
}

/// Result of the XML printer deleting destructor at `0x006754a0`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W14a03XmlPrinterTeardown {
    /// Object offsets whose heap block was released, in release order.
    pub released_offsets: Vec<u16>,
    /// Vtable left in `+0x00`.
    pub final_vtable: u32,
    /// Whether `operator delete(this, 0xf0)` runs.
    pub delete_self: bool,
    /// Allocation size passed to `operator delete`.
    pub delete_size: usize,
}

/// `0x00675450`: non-deleting `tinyxml2::XMLPrinter` destructor.
///
/// The body stores the printer vtable `0x00d3e2ec`, frees the `+0xd0` block
/// when it is not the `+0xd4` inline buffer, frees the `+0x08` block when it
/// is not the `+0x0c` inline buffer, then stores the visitor vtable
/// `0x00d3e2c4`. Returns the released object offsets in order.
pub fn w14a03_xml_printer_destroy(state: &mut W14a03XmlPrinterState) -> Vec<u16> {
    state.vtable = 0x00d3_e2ec;
    let mut released = Vec::new();
    if matches!(state.block_d0, W14a03XmlBlock::Heap(_)) {
        state.block_d0 = W14a03XmlBlock::Inline;
        released.push(0xd0);
    }
    if matches!(state.block_08, W14a03XmlBlock::Heap(_)) {
        state.block_08 = W14a03XmlBlock::Inline;
        released.push(0x08);
    }
    state.vtable = 0x00d3_e2c4;
    released
}

/// `0x006754a0`: scalar deleting destructor of `tinyxml2::XMLPrinter`.
///
/// Runs the destructor above and, when bit 0 of `flags` is set, frees the
/// `0xf0`-byte object itself.
pub fn w14a03_xml_printer_delete(
    state: &mut W14a03XmlPrinterState,
    flags: u8,
) -> W14a03XmlPrinterTeardown {
    let released_offsets = w14a03_xml_printer_destroy(state);
    W14a03XmlPrinterTeardown {
        released_offsets,
        final_vtable: state.vtable,
        delete_self: flags & 1 != 0,
        delete_size: 0xf0,
    }
}

/// `0x00674970`: non-deleting `WritableBufferTextureFactory` destructor.
///
/// The body installs vtables `0x00d3e268` / `0x00d3e284`, releases the two
/// singly-linked node lists at `+0x28` and `+0x14` (each node's `+0x04`
/// payload first, then the node), calls the base destructor at `+0x04`, and
/// leaves the `0x00d3e24c` vtable in `+0x00`.
pub fn w14a03_writable_factory_destroy(state: &mut W14a03FactoryState) -> W14a03FactoryTeardown {
    state.vtable_0 = 0x00d3_e268;
    state.vtable_4 = 0x00d3_e284;
    let released_28 = core::mem::take(&mut state.list_28);
    let released_14 = core::mem::take(&mut state.list_14);
    state.vtable_0 = 0x00d3_e24c;
    W14a03FactoryTeardown {
        released_28,
        released_14,
        calls_base_destructor_at_04: true,
        final_vtable_0: state.vtable_0,
    }
}

/// `0x00674af0`: sibling `WritableBufferTextureFactory` destructor.
///
/// The shipped body is byte-for-byte the same teardown as
/// [`w14a03_writable_factory_destroy`], reached through a different class
/// vtable slot.
pub fn w14a03_writable_factory_destroy_sibling(
    state: &mut W14a03FactoryState,
) -> W14a03FactoryTeardown {
    w14a03_writable_factory_destroy(state)
}

/// Two intrusive lists owned by the writable texture factory.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W14a03FactoryState {
    /// `+0x00`: primary vtable.
    pub vtable_0: u32,
    /// `+0x04`: secondary vtable.
    pub vtable_4: u32,
    /// `+0x14`: second list, head to tail.
    pub list_14: Vec<u32>,
    /// `+0x28`: first list, head to tail.
    pub list_28: Vec<u32>,
}

/// Result of the writable-factory teardown.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W14a03FactoryTeardown {
    /// Payloads released from the `+0x28` list, head to tail.
    pub released_28: Vec<u32>,
    /// Payloads released from the `+0x14` list, head to tail.
    pub released_14: Vec<u32>,
    /// The base destructor at `+0x04` runs after both lists are drained.
    pub calls_base_destructor_at_04: bool,
    /// Vtable left in `+0x00`.
    pub final_vtable_0: u32,
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests_wave14_agent03 {
    use super::*;

    fn key(first: u32, second: u32) -> W14a03MapKey {
        W14a03MapKey { first, second }
    }

    fn tree7() -> W14a03MapTree {
        let mut nodes: Vec<W14a03MapNode> = (0..7)
            .map(|i| W14a03MapNode {
                key: key(i, i),
                is_black: true,
                ..W14a03MapNode::default()
            })
            .collect();
        nodes[3].left = Some(1);
        nodes[3].right = Some(5);
        nodes[1].parent = Some(3);
        nodes[1].left = Some(0);
        nodes[1].right = Some(2);
        nodes[5].parent = Some(3);
        nodes[5].left = Some(4);
        nodes[5].right = Some(6);
        nodes[0].parent = Some(1);
        nodes[2].parent = Some(1);
        nodes[4].parent = Some(5);
        nodes[6].parent = Some(5);
        W14a03MapTree {
            nodes,
            root: Some(3),
            leftmost: Some(0),
            rightmost: Some(6),
            ..W14a03MapTree::default()
        }
    }

    #[test]
    fn find_char_matches_the_shipped_scan() {
        let bytes = b"abca";
        assert_eq!(w14a03_find_char(bytes, 0, b'a'), Some(0));
        assert_eq!(w14a03_find_char(bytes, 1, b'a'), Some(3));
        assert_eq!(w14a03_find_char(bytes, 4, b'a'), None);
        assert_eq!(w14a03_find_char(bytes, 5, b'a'), None);
        assert_eq!(w14a03_find_char(b"", 0, b'a'), None);
        assert_eq!(w14a03_find_char(bytes, 2, b'z'), None);
    }

    #[test]
    fn field_helpers_cover_offsets_and_short_slices() {
        let mut state = [0u32; 22];
        assert!(w14a03_write_fields_50_54(&mut state, 11, 22));
        assert_eq!(state[0x14], 11);
        assert_eq!(state[0x15], 22);
        assert!(!w14a03_write_fields_50_54(&mut state[..21], 1, 2));

        let mut bytes = [0u8; 0x50];
        bytes[0x4d] = 0x7e;
        assert_eq!(w14a03_byte_4d(&bytes), Some(0x7e));
        assert_eq!(w14a03_byte_4d(&bytes[..0x4d]), None);
    }

    #[test]
    fn surface_constructor_zeroes_width_times_height() {
        let surface = w14a03_surface_new(4, 3, 7, 1);
        assert_eq!(surface.width, 4);
        assert_eq!(surface.height, 3);
        assert_eq!(surface.field_08, 7);
        assert_eq!(surface.mode, 1);
        assert!(!surface.initialized);
        assert!(!surface.dirty);
        assert_eq!(surface.pixels, vec![0; 12]);

        let empty = w14a03_surface_new(0, 9, 0, 0);
        assert!(empty.pixels.is_empty());
        assert_eq!(u32::MAX.wrapping_mul(2), 0xffff_fffe);
    }

    #[test]
    fn surface_queue_and_descriptor_round_trip() {
        let mut surface = w14a03_surface_new(4, 3, 9, 1);
        let rect = W14a03Rect {
            x: 1,
            y: 2,
            width: 2,
            height: 1,
        };
        assert_eq!(w14a03_surface_push_rect(&mut surface, rect), 0);
        assert_eq!(w14a03_surface_push_rect(&mut surface, rect), 1);
        assert_eq!(surface.rects.len(), 2);

        let descriptor = w14a03_blit_descriptor(&mut surface);
        assert!(surface.dirty);
        assert_eq!(descriptor.x, 0);
        assert_eq!(descriptor.y, 0);
        assert_eq!(descriptor.width, 4);
        assert_eq!(descriptor.height, 3);
        assert!(descriptor.pixels_available);
        assert_eq!(descriptor.stride, 4);
    }

    #[test]
    fn present_call_requires_the_created_flag() {
        let rect = W14a03Rect {
            x: 1,
            y: 2,
            width: 3,
            height: 4,
        };
        let mut surface = w14a03_surface_new(8, 8, 0, 0);
        assert_eq!(w14a03_present_call(&surface, rect), None);
        surface.initialized = true;
        let call = w14a03_present_call(&surface, rect).unwrap();
        assert_eq!(call.rect, rect);
        assert_eq!(call.blend_mode, 0x16);
        assert_eq!(call.color, [1.0; 4]);
        assert_eq!(call.flags, 0);
    }

    #[test]
    fn set_surface_mode_draws_only_when_ready() {
        let mut surface = w14a03_surface_new(2, 2, 7, 0);
        assert_eq!(w14a03_set_surface_mode(&mut surface, 2, false), None);
        assert_eq!(surface.mode, 2);
        let call = w14a03_set_surface_mode(&mut surface, 1, true).unwrap();
        assert_eq!(call.field_08, 7);
        assert!(call.use_source_alpha);
        assert_eq!(call.flags, 0);
        let call = w14a03_set_surface_mode(&mut surface, 2, true).unwrap();
        assert!(!call.use_source_alpha);
    }

    #[test]
    fn flush_plan_emits_nonempty_rects_and_clears_the_queue() {
        let mut surface = w14a03_surface_new(4, 4, 3, 1);
        w14a03_surface_push_rect(
            &mut surface,
            W14a03Rect {
                x: 1,
                y: 1,
                width: 2,
                height: 2,
            },
        );
        w14a03_surface_push_rect(
            &mut surface,
            W14a03Rect {
                x: 3,
                y: 3,
                width: 0,
                height: 2,
            },
        );
        let plan = w14a03_flush_surface(&mut surface, true);
        assert!(plan.create_surface);
        assert!(plan.initialized);
        assert!(plan.cleared_dirty);
        assert_eq!(plan.blits.len(), 1);
        let blit = plan.blits[0];
        assert_eq!(blit.source_index, 5);
        assert_eq!(blit.pitch, 4);
        assert_eq!(blit.field_08, 3);
        assert!(blit.use_source_alpha);
        assert_eq!(blit.flags, 0);
        assert!(surface.rects.is_empty());
        assert!(!surface.dirty);

        let empty_plan = w14a03_flush_surface(&mut surface, true);
        assert!(!empty_plan.create_surface);
        assert!(empty_plan.blits.is_empty());
        assert!(empty_plan.cleared_dirty);
    }

    #[test]
    fn failed_flush_keeps_the_queue_and_clears_dirty() {
        let mut surface = w14a03_surface_new(2, 2, 0, 0);
        surface.initialized = true;
        surface.dirty = true;
        w14a03_surface_push_rect(
            &mut surface,
            W14a03Rect {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            },
        );
        let plan = w14a03_flush_surface(&mut surface, false);
        assert!(!plan.create_surface);
        assert!(!plan.initialized);
        assert!(plan.blits.is_empty());
        assert!(plan.cleared_dirty);
        assert_eq!(surface.rects.len(), 1);
        assert!(!surface.dirty);
    }

    #[test]
    fn lower_bound_uses_second_then_first_order() {
        let tree = tree7();
        assert_eq!(w14a03_tree_lower_bound(&tree, key(3, 3)), Some(3));
        assert_eq!(w14a03_tree_lower_bound(&tree, key(2, 3)), Some(3));
        assert_eq!(w14a03_tree_lower_bound(&tree, key(4, 3)), Some(4));
        assert_eq!(w14a03_tree_lower_bound(&tree, key(0, 0)), Some(0));
        assert_eq!(w14a03_tree_lower_bound(&tree, key(0, 7)), None);
        assert_eq!(
            w14a03_tree_lower_bound(&W14a03MapTree::default(), key(1, 1)),
            None
        );
    }

    #[test]
    fn insert_hint_selects_the_recovered_links() {
        let tree = tree7();
        assert_eq!(
            w14a03_tree_insert_hint(&W14a03MapTree::default(), key(1, 1)).link,
            W14a03InsertLink::HeaderLeft
        );
        assert_eq!(
            w14a03_tree_insert_hint(&tree, key(0, 0)),
            W14a03InsertHint {
                node: Some(0),
                link: W14a03InsertLink::Existing
            }
        );
        let single = W14a03MapTree {
            nodes: vec![W14a03MapNode {
                key: key(1, 1),
                is_black: true,
                ..W14a03MapNode::default()
            }],
            root: Some(0),
            leftmost: Some(0),
            rightmost: Some(0),
            ..W14a03MapTree::default()
        };
        assert_eq!(
            w14a03_tree_insert_hint(&single, key(0, 1)),
            W14a03InsertHint {
                node: None,
                link: W14a03InsertLink::HeaderLeft
            }
        );
        assert_eq!(
            w14a03_tree_insert_hint(&tree, key(7, 7)),
            W14a03InsertHint {
                node: Some(6),
                link: W14a03InsertLink::Right
            }
        );
        assert_eq!(
            w14a03_tree_insert_hint(&tree, key(2, 2)),
            W14a03InsertHint {
                node: Some(2),
                link: W14a03InsertLink::Existing
            }
        );
        assert_eq!(
            w14a03_tree_insert_hint(&tree, key(3, 2)),
            W14a03InsertHint {
                node: Some(2),
                link: W14a03InsertLink::Right
            }
        );
        assert_eq!(
            w14a03_tree_insert_hint(&tree, key(2, 1)),
            W14a03InsertHint {
                node: None,
                link: W14a03InsertLink::HeaderLeft
            }
        );
    }

    #[test]
    fn mark_all_walks_every_entry_in_order() {
        let mut tree = tree7();
        assert_eq!(w14a03_tree_mark_all(&mut tree), 7);
        assert!(tree.nodes.iter().all(|node| node.marked));
        let mut empty = W14a03MapTree::default();
        assert_eq!(w14a03_tree_mark_all(&mut empty), 0);
    }

    #[test]
    fn clone_subtree_copies_links_and_values() {
        let mut tree = tree7();
        tree.nodes[2].surface = Some(5);
        tree.nodes[2].marked = true;
        let clone = w14a03_tree_clone_subtree(&mut tree, Some(3), None).unwrap();
        assert_eq!(clone, 7);
        assert_eq!(tree.nodes.len(), 14);
        assert_eq!(tree.nodes[clone].key, key(3, 3));
        assert_eq!(tree.nodes[clone].parent, None);
        let left = tree.nodes[clone].left.unwrap();
        let right = tree.nodes[clone].right.unwrap();
        assert_eq!(tree.nodes[left].key, key(1, 1));
        assert_eq!(tree.nodes[right].key, key(5, 5));
        assert_eq!(tree.nodes[left].parent, Some(clone));
        // Source subtree is unchanged.
        assert_eq!(tree.nodes[3].key, key(3, 3));
        assert_eq!(tree.nodes[3].parent, None);

        let mut leaf = W14a03MapTree::default();
        assert_eq!(w14a03_tree_clone_subtree(&mut leaf, None, None), None);
        assert_eq!(leaf.nodes.len(), 0);
    }

    #[test]
    fn map_operations_use_the_lower_bound_entry() {
        let mut tree = tree7();
        tree.nodes[2].surface = Some(0);
        tree.nodes[3].surface = Some(1);
        tree.nodes[5].surface = Some(2);
        let mut surfaces = vec![
            w14a03_surface_new(2, 2, 1, 0),
            w14a03_surface_new(3, 3, 2, 0),
            w14a03_surface_new(4, 4, 3, 0),
        ];
        let rect = W14a03Rect {
            x: 1,
            y: 1,
            width: 1,
            height: 1,
        };

        // Exact key -> node 2's surface.
        assert!(w14a03_tree_append_rect(
            &mut tree,
            key(2, 2),
            rect,
            &mut surfaces
        ));
        assert_eq!(tree.counter_5c, 1);
        assert_eq!(surfaces[0].rects, vec![rect]);

        // Missing key between nodes 2 and 3 -> lower bound node 3's surface.
        assert!(w14a03_tree_append_rect(
            &mut tree,
            key(2, 3),
            rect,
            &mut surfaces
        ));
        assert_eq!(tree.counter_5c, 2);
        assert_eq!(surfaces[1].rects, vec![rect]);

        assert_eq!(
            w14a03_tree_entry_pixels(&tree, key(2, 2), &surfaces).map(<[u32]>::len),
            Some(4)
        );
        assert_eq!(
            w14a03_tree_entry_pixels(&tree, key(2, 3), &surfaces).map(<[u32]>::len),
            Some(9)
        );
        assert_eq!(w14a03_tree_entry_pixels(&tree, key(0, 7), &surfaces), None);
        assert_eq!(w14a03_tree_flush_target(&tree, key(3, 3)), Some(1));
        assert_eq!(w14a03_tree_flush_target(&tree, key(5, 5)), Some(2));
        assert_eq!(w14a03_tree_flush_target(&tree, key(4, 4)), None);
        assert_eq!(w14a03_tree_flush_target(&tree, key(0, 7)), None);
    }

    #[test]
    fn bulk_mode_update_returns_entries_in_tree_order() {
        let mut tree = tree7();
        for (entry, surface) in [(0usize, 4usize), (2, 1), (4, 3), (6, 2)] {
            tree.nodes[entry].surface = Some(surface);
        }
        let targets = w14a03_tree_set_mode_all(&mut tree, 7);
        assert_eq!(tree.field_58, 7);
        let entries: Vec<usize> = targets.iter().map(|target| target.entry).collect();
        assert_eq!(entries, vec![0, 2, 4, 6]);
        let surfaces: Vec<usize> = targets.iter().map(|target| target.surface).collect();
        assert_eq!(surfaces, vec![4, 1, 3, 2]);
    }

    #[test]
    fn xml_printer_teardown_releases_heap_blocks_in_order() {
        let mut state = W14a03XmlPrinterState {
            vtable: 0,
            block_08: W14a03XmlBlock::Heap(vec![1, 2, 3]),
            block_d0: W14a03XmlBlock::Heap(vec![4, 5]),
        };
        let released = w14a03_xml_printer_destroy(&mut state);
        assert_eq!(released, vec![0xd0, 0x08]);
        assert_eq!(state.block_08, W14a03XmlBlock::Inline);
        assert_eq!(state.block_d0, W14a03XmlBlock::Inline);
        assert_eq!(state.vtable, 0x00d3_e2c4);

        let mut inline_state = W14a03XmlPrinterState::default();
        assert!(w14a03_xml_printer_destroy(&mut inline_state).is_empty());

        let mut deleting = W14a03XmlPrinterState {
            vtable: 0,
            block_08: W14a03XmlBlock::Heap(vec![1]),
            block_d0: W14a03XmlBlock::Inline,
        };
        let plan = w14a03_xml_printer_delete(&mut deleting, 1);
        assert_eq!(plan.released_offsets, vec![0x08]);
        assert_eq!(plan.final_vtable, 0x00d3_e2c4);
        assert!(plan.delete_self);
        assert_eq!(plan.delete_size, 0xf0);
        assert!(!w14a03_xml_printer_delete(&mut deleting, 0).delete_self);
    }

    #[test]
    fn visitor_and_browser_vtables_and_delete_plans() {
        let mut vtable = 0u32;
        w14a03_install_xml_visitor_vtable(&mut vtable);
        assert_eq!(vtable, 0x00d3_e2c4);
        let plan = w14a03_xml_visitor_delete_plan(&mut vtable, 1);
        assert_eq!(vtable, 0x00d3_e2c4);
        assert!(plan.delete_self);
        assert_eq!(plan.delete_size, 4);
        assert!(!w14a03_xml_visitor_delete_plan(&mut vtable, 0).delete_self);

        let mut browser = 0u32;
        w14a03_install_browser_texture_factory_vtable(&mut browser);
        assert_eq!(browser, 0x00d3_e24c);
        let plan = w14a03_browser_factory_delete_plan(1);
        assert_eq!(plan.final_vtable, 0x00d3_e24c);
        assert!(plan.delete_self);
        assert_eq!(plan.delete_size, 4);
        assert!(!w14a03_browser_factory_delete_plan(0).delete_self);
    }

    #[test]
    fn factory_teardown_drains_both_lists() {
        let mut state = W14a03FactoryState {
            vtable_0: 0,
            vtable_4: 0,
            list_14: vec![10, 11],
            list_28: vec![20, 21, 22],
        };
        let plan = w14a03_writable_factory_destroy(&mut state);
        assert_eq!(plan.released_28, vec![20, 21, 22]);
        assert_eq!(plan.released_14, vec![10, 11]);
        assert!(plan.calls_base_destructor_at_04);
        assert_eq!(plan.final_vtable_0, 0x00d3_e24c);
        assert_eq!(state.vtable_4, 0x00d3_e284);
        assert!(state.list_14.is_empty());
        assert!(state.list_28.is_empty());

        let mut sibling = W14a03FactoryState {
            vtable_0: 0,
            vtable_4: 0,
            list_14: vec![7],
            list_28: Vec::new(),
        };
        let plan = w14a03_writable_factory_destroy_sibling(&mut sibling);
        assert_eq!(plan.released_14, vec![7]);
        assert!(plan.released_28.is_empty());
    }
}
