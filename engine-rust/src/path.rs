//! Deterministic grid pathfinding.
//!
//! StarCraft uses a two-level pathfinder: coarse pathfinding regions on the
//! 32-pixel tile grid and a short-range grid search over 8x8-pixel walk
//! cells.  The region half follows the recovered builder at `0x7a302c10`
//! (tile resolution, bbox centres, adjacency and the size-6 merge pass); the
//! short-range half is an octile A* over the recovered tileset walk grid with
//! unit footprints and building obstacles.  The module is deliberately
//! dependency-free and deterministic so simulations are reproducible.
//!
//! Tie-breaking follows the recovered retail open-list heap.  `FUN_7a305590`
//! is the region-graph A*: its open list is a binary heap ordered **only** by
//! the `f` field at `+0x14` of a heap entry.  `FUN_7a305e20` (sift-up) keeps
//! bubbling while the new entry's `f` is less than *or equal to* its parent's,
//! and `FUN_7a305da0` (sift-down) stops as soon as the parent is less than or
//! equal to the chosen child and chooses the left child on a tie.  Equal-`f`
//! entries therefore pop newest-first instead of the old `(f,g,x,y)` order.
//! Neighbours are visited in the recovered `local_18` order from
//! `FUN_7a304c10`: left, up, right, down, up-left, up-right, down-left,
//! down-right.  The decompile is stored in
//! `docs/evidence/pathfinding_astar.txt`.

use crate::map::{Terrain, WALK_TILE_PX};

pub const STRAIGHT_COST: i32 = 10;
pub const DIAGONAL_COST: i32 = 14;

/// The retail distance helper `FUN_7a1a4060` (runtime `0x7a1a4060`).
///
/// The decompile is:
///
/// ```c
/// dx = abs(x2-x1); dy = abs(y2-y1);
/// if (dx < dy) { a = dx; b = dy; } else { a = dy; b = dx; }
/// if (b >> 2 < a) {
///     a = (a >> 1) + a;
///     return (a >> 2) + ((a >> 7) - (b >> 6)) - (b >> 4) + b;
/// }
/// return b;
/// ```
///
/// It is the exact octile approximation used by both the region A* and the
/// movement code.  The integer expression deliberately reproduces the
/// decompiled shifts so a Rust unit test can pin the recovered values.
pub fn retail_distance(a: (i32, i32), b: (i32, i32)) -> i32 {
    let dx = (a.0 - b.0).abs();
    let dy = (a.1 - b.1).abs();
    let (mut lo, hi) = if dx < dy { (dx, dy) } else { (dy, dx) };
    if hi >> 2 < lo {
        lo = (lo >> 1) + lo;
        return (lo >> 2) + ((lo >> 7) - (hi >> 6)) - (hi >> 4) + hi;
    }
    hi
}

/// The recovered retail open list.
///
/// `FUN_7a305590` keeps one heap entry per region and compares only the
/// 32-bit `f` at `+0x14`.  This is a faithful 0-based port of the two heap
/// helpers:
///
/// * `FUN_7a305e20` sift-up: stop when `f(parent) < f(child)`, so equal-cost
///   children bubble above their parents;
/// * `FUN_7a305da0` sift-down: choose the right child only when
///   `f(right) < f(left)` (ties keep the left child) and stop when
///   `f(parent) <= f(chosen)`.
///
/// This differs from `BinaryHeap<Reverse<(f, g, x, y)>>`, which ordered ties by
/// `g` and coordinate.  The recovered heap makes the order depend only on
/// insertion/discovery order, which in turn is the recovered neighbour order.
#[derive(Clone, Debug, Default)]
struct RetailOpen {
    /// Heap of node indices.
    heap: Vec<u32>,
    /// Position of each node in `heap`, or `u32::MAX`.
    pos: Vec<u32>,
    /// Cached `f` value for each node.
    f: Vec<i32>,
}

impl RetailOpen {
    fn new(nodes: usize) -> Self {
        Self {
            heap: Vec::new(),
            pos: vec![u32::MAX; nodes],
            f: vec![0; nodes],
        }
    }

    fn sift_up(&mut self, mut i: usize) {
        while i > 0 {
            let p = (i - 1) / 2;
            // Recovered: stop only when the parent is strictly cheaper.
            if self.f[self.heap[p] as usize] < self.f[self.heap[i] as usize] {
                break;
            }
            self.heap.swap(p, i);
            self.pos[self.heap[p] as usize] = p as u32;
            self.pos[self.heap[i] as usize] = i as u32;
            i = p;
        }
    }

    fn sift_down(&mut self, mut i: usize) {
        let n = self.heap.len();
        loop {
            let left = i * 2 + 1;
            if left >= n {
                break;
            }
            let right = left + 1;
            let mut child = left;
            if right < n && self.f[self.heap[right] as usize] < self.f[self.heap[left] as usize] {
                child = right;
            }
            if self.f[self.heap[i] as usize] <= self.f[self.heap[child] as usize] {
                break;
            }
            self.heap.swap(i, child);
            self.pos[self.heap[i] as usize] = i as u32;
            self.pos[self.heap[child] as usize] = child as u32;
            i = child;
        }
    }

    /// Insert a node, or lower its `f` and sift it up.  A node that is already
    /// closed is re-opened by the caller before this call.
    fn push_or_decrease(&mut self, node: u32, f: i32) {
        let ni = node as usize;
        if self.pos[ni] == u32::MAX {
            self.f[ni] = f;
            self.heap.push(node);
            let i = self.heap.len() - 1;
            self.pos[ni] = i as u32;
            self.sift_up(i);
        } else {
            self.f[ni] = f;
            let i = self.pos[ni] as usize;
            self.sift_up(i);
        }
    }

    fn pop_min(&mut self) -> Option<u32> {
        let last = self.heap.pop()?;
        if self.heap.is_empty() {
            self.pos[last as usize] = u32::MAX;
            return Some(last);
        }
        let root = self.heap[0];
        self.heap[0] = last;
        self.pos[last as usize] = 0;
        self.pos[root as usize] = u32::MAX;
        self.sift_down(0);
        Some(root)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn from_center(cx: i32, cy: i32, half_w: i32, half_h: i32) -> Self {
        Self::new(
            cx.saturating_sub(half_w),
            cy.saturating_sub(half_h),
            cx.saturating_add(half_w),
            cy.saturating_add(half_h),
        )
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.left < other.right
            && other.left < self.right
            && self.top < other.bottom
            && other.top < self.bottom
    }

    pub fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }

    /// `0x009308d0`: two intervals of length `len` overlap.
    pub fn ranges_overlap(a: i32, b: i32, len: i32) -> bool {
        b < a + len && a < b + len
    }

    /// `0x00965210`: inclusive bounds test used by the retail point query.
    pub fn contains_point_inclusive(&self, x: i32, y: i32) -> bool {
        x >= self.left && x <= self.right && y >= self.top && y <= self.bottom
    }
}

#[derive(Clone, Debug)]
pub struct PathRequest {
    pub start: (i32, i32),
    pub goal: (i32, i32),
    /// Half extent of the moving unit's footprint, in pixels.
    pub half_width: i32,
    pub half_height: i32,
    /// Flyers ignore terrain and obstacles.
    pub flying: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Path {
    /// Waypoints in pixels (center of each 8-pixel cell), excluding the
    /// starting position and ending at the requested goal cell.
    pub waypoints: Vec<(i32, i32)>,
    pub cost: i32,
}

mod obstacles;
pub use obstacles::Obstacles;

/// One pathfinding region from the retail builder.
#[derive(Clone, Debug, Default)]
pub struct Region {
    pub id: u32,
    /// Number of map tiles in the region (retail keeps this in the region
    /// header and the AI `region_size` command reads it).
    pub size: u32,
    pub min_tile: (i32, i32),
    pub max_tile: (i32, i32),
    /// 8-connected neighbours in recovered append order (not sorted).  The
    /// order is observable because the A* open-list tie-break follows
    /// discovery order.
    pub neighbours: Vec<u32>,
}

impl Region {
    /// Region centre in 8-pixel walk cells.  The retail builder stores the
    /// bounding-box midpoint (`0x7a303840`); tiles are 32 pixels, so the
    /// midpoint tile maps to walk cell `tile * 4 + 2`.
    pub fn centre_cell(&self) -> (i32, i32) {
        let cx = (self.min_tile.0 + self.max_tile.0) / 2;
        let cy = (self.min_tile.1 + self.max_tile.1) / 2;
        (cx * 4 + 2, cy * 4 + 2)
    }
}

/// Region graph built the way the retail builder does it.
///
/// `0x7a302c10` operates on a **tile grid** with a 256-cell row stride
/// (`grid[y*0x100 + x]`), not on the 8-pixel walk grid: the 16 minitile
/// flags of each tile are used only to decide whether two tiles touch.  A
/// region stores its bounding box and centre, and the adjacency lists are
/// built by `0x7a3046b0` / `0x7a303500`.  `0x7a303fd0` then merges regions
/// below an increasing size threshold into their smallest neighbour; the
/// same pass is implemented here (thresholds 6, 8, ..., 64).
/// The recovered walk-grid table holds one byte per 8-pixel minitile:
/// `0x80` when the matching `vf4` walkable bit is clear and `0x00` otherwise.
/// The client builds it from the tileset's 16 minitile flags.
///
/// `FUN_7a3046b0` classifies a neighbouring tile as connected using that
/// grid.  Orthogonal neighbours are accepted unconditionally (cases 0..=3
/// branch straight to the append code).  A diagonal neighbour (cases 4..=7)
/// needs the two 2x2 corner blocks that touch the shared corner — the current
/// tile's corner and the diagonal tile's opposite corner — and then at least
/// one of the two side tiles' corner blocks.  The offsets below are the exact
/// `pcVar1 + 0/1/0x400/...` reads from the decompile.
pub fn retail_tile_adjacent(terrain: &Terrain, x: i32, y: i32, dx: i32, dy: i32) -> bool {
    if dx == 0 && dy == 0 {
        return false;
    }
    if dx == 0 || dy == 0 {
        return true;
    }
    let open2 = |ox: i32, oy: i32| {
        (0..2).all(|oy2| (0..2).all(|ox2| terrain.walkable_cell(ox + ox2, oy + oy2)))
    };
    let bx = x * 4;
    let by = y * 4;
    let corner_x = if dx > 0 { 2 } else { 0 };
    let corner_y = if dy > 0 { 2 } else { 0 };
    let diag_x = if dx > 0 { 4 } else { -2 };
    let diag_y = if dy > 0 { 4 } else { -2 };
    let side_h = (bx + diag_x, by + corner_y);
    let side_v = (bx + corner_x, by + diag_y);
    open2(bx + corner_x, by + corner_y)
        && open2(bx + diag_x, by + diag_y)
        && (open2(side_h.0, side_h.1) || open2(side_v.0, side_v.1))
}

#[derive(Clone, Debug, Default)]
pub struct RegionMap {
    /// Tile dimensions (the retail 256-stride grid).
    pub width: i32,
    pub height: i32,
    labels: Vec<u32>,
    regions: Vec<Region>,
    count: u32,
}

impl RegionMap {
    pub fn from_terrain(terrain: &Terrain) -> Self {
        let width = terrain.width as i32;
        let height = terrain.height as i32;
        let n = (width * height) as usize;
        let mut labels = vec![u32::MAX; n];
        let mut stack: Vec<(i32, i32)> = Vec::new();

        let tile_walkable = |tx: i32, ty: i32| -> bool {
            if tx < 0 || ty < 0 || tx >= width || ty >= height {
                return false;
            }
            for my in 0..4 {
                for mx in 0..4 {
                    if terrain.walkable_cell(tx * 4 + mx, ty * 4 + my) {
                        return true;
                    }
                }
            }
            false
        };
        // Retail region creation (`FUN_00763840`, 32-bit dump, 2026-11-06):
        // unassigned walkable tiles carry a group label (>= 5000; here
        // walkability + ground height, INFERENCE); regions are seeded from
        // the largest rectangle of one group found among up to 25 candidates
        // (`FUN_00764d60`: at most 8 x 7 tiles, aspect <= 3), accepted once
        // its area reaches the previous region's area (56 at first), grown
        // one tile around (`FUN_00764360`, two passes), and every region
        // grows once more when the rectangles drop below 7 tiles.  The old
        // flood fill made each connected area one region (Boot Camp: 1).
        // Group labels sit far above any region id (retail uses >= 5000
        // and aborts above 5000 regions).
        const GROUP: u32 = 0x4000_0000;
        for (i, l) in labels.iter_mut().enumerate() {
            let (tx, ty) = (i as i32 % width, i as i32 / width);
            if tile_walkable(tx, ty) {
                *l = GROUP + terrain.tile_height_flags(tx, ty) as u32;
            }
        }
        let idx = |x: i32, y: i32| (y * width + x) as usize;
        let rect_at = |labels: &Vec<u32>, x: i32, y: i32, g: u32| -> (i32, i32, i32, i32) {
            let max_x = (x + 8).min(width);
            let max_y = (y + 7).min(height);
            let (mut x1, mut y1) = (x + 1, y + 1);
            let (mut right, mut down, mut corner_prev) = (true, true, true);
            while x1 < max_x && y1 < max_y {
                if right && (y..y1).any(|yy| labels[idx(x1, yy)] != g) {
                    right = false;
                }
                if down && (x..x1).any(|xx| labels[idx(xx, y1)] != g) {
                    down = false;
                }
                if !right && !down {
                    break;
                }
                let corner = labels[idx(x1, y1)] == g && corner_prev;
                if corner {
                    if down {
                        y1 += 1;
                    }
                    if right {
                        x1 += 1;
                    }
                } else if down {
                    y1 += 1;
                } else if right {
                    x1 += 1;
                }
                corner_prev = corner;
            }
            let (mut w, mut h) = (x1 - x, y1 - y);
            if h < w {
                w = w.min(h * 3);
            } else if w * 3 < h {
                h = w * 3;
            }
            (x, y, x + w, y + h)
        };
        let grow = |labels: &mut Vec<u32>, b: (i32, i32, i32, i32), id: u32, g: u32| {
            for _ in 0..2 {
                for y in b.1..b.3 {
                    for x in b.0..b.2 {
                        if labels[idx(x, y)] != g {
                            continue;
                        }
                        let near = (y > 0 && labels[idx(x, y - 1)] == id)
                            || (x > 0 && labels[idx(x - 1, y)] == id)
                            || (x + 1 < width && labels[idx(x + 1, y)] == id)
                            || (y + 1 < height && labels[idx(x, y + 1)] == id);
                        if near {
                            labels[idx(x, y)] = id;
                        }
                    }
                }
            }
        };
        let expand = |b: (i32, i32, i32, i32)| {
            (
                (b.0 - 1).max(0),
                (b.1 - 1).max(0),
                (b.2 + 1).min(width),
                (b.3 + 1).min(height),
            )
        };
        let mut boxes: Vec<((i32, i32, i32, i32), u32)> = Vec::new();
        let mut threshold = 0x38i32;
        let mut resume = 0usize;
        let mut grown_once = false;
        loop {
            let mut best: Option<((i32, i32, i32, i32), u32)> = None;
            let mut best_area = 0;
            let mut tries = 0;
            let mut k = 0usize;
            while k < n {
                let i = (resume + k) % n;
                let g = labels[i];
                if g >= GROUP && g != u32::MAX {
                    let (x, y) = (i as i32 % width, i as i32 / width);
                    let r = rect_at(&labels, x, y, g);
                    tries += 1;
                    let area = (r.2 - r.0) * (r.3 - r.1);
                    if area > best_area {
                        best_area = area;
                        best = Some((r, g));
                    }
                    if best_area >= threshold || tries > 25 {
                        break;
                    }
                    k += (r.2 - x).max(1) as usize;
                    continue;
                }
                k += 1;
            }
            let Some((r, g)) = best else {
                break;
            };
            threshold = best_area;
            let id = boxes.len() as u32;
            for y in r.1..r.3 {
                for x in r.0..r.2 {
                    labels[idx(x, y)] = id;
                }
            }
            let b = expand(r);
            grow(&mut labels, b, id, g);
            boxes.push((b, g));
            resume = idx(r.2.min(width - 1), r.1);
            if threshold < 7 && !grown_once {
                grown_once = true;
                for (rid, (bb, gg)) in boxes.iter_mut().enumerate() {
                    *bb = expand(*bb);
                    grow(&mut labels, *bb, rid as u32, *gg);
                }
            }
        }
        for (i, l) in labels.iter_mut().enumerate() {
            let _ = i;
            if *l >= GROUP && (*l as usize) >= boxes.len() {
                *l = u32::MAX;
            }
        }
        let mut regions: Vec<Region> = (0..boxes.len() as u32)
            .map(|id| Region {
                id,
                size: 0,
                min_tile: (i32::MAX, i32::MAX),
                max_tile: (i32::MIN, i32::MIN),
                neighbours: Vec::new(),
            })
            .collect();
        for (i, &l) in labels.iter().enumerate() {
            if let Some(r) = regions.get_mut(l as usize) {
                let (x, y) = (i as i32 % width, i as i32 / width);
                r.size += 1;
                r.min_tile = (r.min_tile.0.min(x), r.min_tile.1.min(y));
                r.max_tile = (r.max_tile.0.max(x), r.max_tile.1.max(y));
            }
        }
        let _ = &mut stack;
        let count = regions.len() as u32;
        let mut map = Self {
            width,
            height,
            labels,
            regions,
            count,
        };
        map.rebuild_adjacency();
        map.merge_small_regions();
        map
    }

    /// Reciprocal adjacency from the current labels.
    fn rebuild_adjacency(&mut self) {
        for r in &mut self.regions {
            r.neighbours.clear();
        }
        let w = self.width;
        for y in 0..self.height {
            for x in 0..self.width {
                let a = self.labels[(y * w + x) as usize];
                if a == u32::MAX {
                    continue;
                }
                for (dx, dy) in [(1, 0), (0, 1), (1, 1), (-1, 1)] {
                    let nx = x + dx;
                    let ny = y + dy;
                    if nx < 0 || ny < 0 || nx >= self.width || ny >= self.height {
                        continue;
                    }
                    let b = self.labels[(ny * w + nx) as usize];
                    if b == u32::MAX || a == b {
                        continue;
                    }
                    if !self.regions[a as usize].neighbours.contains(&b) {
                        self.regions[a as usize].neighbours.push(b);
                    }
                    if !self.regions[b as usize].neighbours.contains(&a) {
                        self.regions[b as usize].neighbours.push(a);
                    }
                }
            }
        }
        for r in &mut self.regions {
            r.neighbours.dedup();
        }
    }

    /// The `0x7a303fd0` merge loop: regions below a growing threshold are
    /// folded into the smallest adjacent region.  A merge rewrites the tile
    /// labels and the region's bounding box, exactly like the retail routine
    /// (`psVar9[2] += psVar7[2]`, bbox union, source marked `0x1fff`).
    ///
    /// The ordinary case is **one pass at size < 6**.  The threshold only
    /// grows (8, 10, ...) while the live region count is still at least
    /// `0x9c4` (2500), the pathological-map guard at the end of the routine.
    fn merge_small_regions(&mut self) {
        let mut threshold = 6u32;
        loop {
            let mut changed = false;
            let mut remap: Vec<u32> = (0..self.regions.len() as u32).collect();
            let mut merged_into: Vec<Option<u32>> = vec![None; self.regions.len()];
            for id in 0..self.regions.len() {
                if merged_into[id].is_some() || self.regions[id].size == 0 {
                    continue;
                }
                if self.regions[id].size >= threshold {
                    continue;
                }
                let mut best: Option<(u32, u32)> = None;
                for &nb in &self.regions[id].neighbours {
                    let target = remap[nb as usize];
                    if target == id as u32 {
                        continue;
                    }
                    let size = self.regions[target as usize].size;
                    if best.is_none_or(|(bs, _)| size < bs) {
                        best = Some((size, target));
                    }
                }
                if let Some((_, target)) = best {
                    let src = id as u32;
                    let dst = target;
                    self.regions[dst as usize].size += self.regions[src as usize].size;
                    self.regions[dst as usize].min_tile.0 = self.regions[dst as usize]
                        .min_tile
                        .0
                        .min(self.regions[src as usize].min_tile.0);
                    self.regions[dst as usize].min_tile.1 = self.regions[dst as usize]
                        .min_tile
                        .1
                        .min(self.regions[src as usize].min_tile.1);
                    self.regions[dst as usize].max_tile.0 = self.regions[dst as usize]
                        .max_tile
                        .0
                        .max(self.regions[src as usize].max_tile.0);
                    self.regions[dst as usize].max_tile.1 = self.regions[dst as usize]
                        .max_tile
                        .1
                        .max(self.regions[src as usize].max_tile.1);
                    self.regions[src as usize].size = 0;
                    for (i, r) in remap.iter_mut().enumerate() {
                        if *r == src {
                            *r = dst;
                        }
                        if merged_into[i] == Some(src) {
                            merged_into[i] = Some(dst);
                        }
                    }
                    merged_into[src as usize] = Some(dst);
                    changed = true;
                }
            }
            if changed {
                for label in &mut self.labels {
                    if *label != u32::MAX {
                        *label = remap[*label as usize];
                    }
                }
                self.rebuild_adjacency();
            }
            let live = self.regions.iter().filter(|r| r.size > 0).count();
            if live < 2500 || threshold >= 64 {
                break;
            }
            if !changed {
                threshold += 2;
            }
        }
        // Compact ids so `count()` and the AI `region_size` lookups stay
        // dense.  Keep dead entries out of the public region list.
        let mut compact = vec![u32::MAX; self.regions.len()];
        let mut next = 0u32;
        for (i, r) in self.regions.iter().enumerate() {
            if r.size > 0 {
                compact[i] = next;
                next += 1;
            }
        }
        let mut out = Vec::with_capacity(next as usize);
        for (i, mut r) in self.regions.iter().cloned().enumerate() {
            if r.size == 0 {
                continue;
            }
            r.id = compact[i];
            r.neighbours = r
                .neighbours
                .iter()
                .filter_map(|&n| {
                    let c = compact[n as usize];
                    (c != u32::MAX).then_some(c)
                })
                .collect();
            r.neighbours.dedup();
            out.push(r);
        }
        for label in &mut self.labels {
            if *label != u32::MAX {
                *label = compact[*label as usize];
            }
        }
        self.regions = out;
        self.count = next;
    }

    pub fn count(&self) -> u32 {
        self.count
    }

    pub fn region(&self, id: u32) -> Option<&Region> {
        self.regions.get(id as usize)
    }

    pub fn regions(&self) -> &[Region] {
        &self.regions
    }

    /// Neighbour region ids in recovered append order (`FUN_0075d6b0`).
    /// The retail helper writes the list pointer and returns its count.
    pub fn neighbours_of(&self, id: u32) -> &[u32] {
        self.regions
            .get(id as usize)
            .map(|r| r.neighbours.as_slice())
            .unwrap_or(&[])
    }

    /// Region centre in pixels.  `FUN_0075d660` reads the two fixed-point
    /// centre fields and shifts them right by 8; our walk-cell centre is
    /// already in 8-pixel units, so the conversion is `centre_cell * 8`.
    pub fn region_centre_px(&self, id: u32) -> Option<(i32, i32)> {
        let (cx, cy) = self.regions.get(id as usize)?.centre_cell();
        Some((cx * 8, cy * 8))
    }

    /// Region id for an 8-pixel walk cell.  The tile grid is four times
    /// coarser, so the cell maps to `cell / 4`.
    pub fn region_at_cell(&self, cell: (i32, i32)) -> Option<u32> {
        self.region_at_tile((cell.0.div_euclid(4), cell.1.div_euclid(4)))
    }

    pub fn region_at_tile(&self, tile: (i32, i32)) -> Option<u32> {
        if tile.0 < 0 || tile.1 < 0 || tile.0 >= self.width || tile.1 >= self.height {
            return None;
        }
        let v = self.labels[(tile.1 * self.width + tile.0) as usize];
        if v == u32::MAX { None } else { Some(v) }
    }

    pub fn connected(&self, a: (i32, i32), b: (i32, i32)) -> bool {
        self.region_path(a, b).is_some()
    }

    /// Exact retail region path: A* over the recovered region graph.
    ///
    /// `FUN_7a305590` searches the region adjacency lists directly (one node
    /// per region) and compares the region-centre distance from
    /// `FUN_7a1a4060`.  The open list is `RetailOpen`, so ties are resolved by
    /// discovery order and the heap rules recovered from `FUN_7a305e20` /
    /// `FUN_7a305da0`, not by a secondary `g`/coordinate key.  `FUN_7a305590`
    /// also stops after 125 open-list entries (`0x7d`); the same bound is
    /// applied here.
    pub fn region_path(&self, a: (i32, i32), b: (i32, i32)) -> Option<Vec<u32>> {
        let start = self.region_at_cell(a)?;
        let goal = self.region_at_cell(b)?;
        if start == goal {
            return Some(vec![start]);
        }
        let n = self.regions.len();
        let mut g = vec![i32::MAX; n];
        let mut parent = vec![u32::MAX; n];
        let mut closed = vec![false; n];
        let mut open = RetailOpen::new(n);
        let goal_centre = self.regions[goal as usize].centre_cell();
        g[start as usize] = 0;
        let h = retail_distance(self.regions[start as usize].centre_cell(), goal_centre);
        open.push_or_decrease(start, h);

        let mut found = false;
        let mut pops = 0usize;
        while let Some(cur) = open.pop_min() {
            if cur == goal {
                found = true;
                break;
            }
            closed[cur as usize] = true;
            pops += 1;
            if pops >= 125 {
                break;
            }
            let cur_centre = self.regions[cur as usize].centre_cell();
            // `neighbours` preserves the recovered append order.  It is the
            // order in which `FUN_7a3046b0` walks `local_18` (left, up, right,
            // down, up-left, up-right, down-left, down-right) and therefore
            // the order the retail heap sees new nodes.
            for &nb in self.neighbours_of(cur) {
                if closed[nb as usize] {
                    continue;
                }
                let step = retail_distance(cur_centre, self.regions[nb as usize].centre_cell());
                let tentative = g[cur as usize].saturating_add(step);
                if tentative < g[nb as usize] {
                    g[nb as usize] = tentative;
                    parent[nb as usize] = cur;
                    let h = retail_distance(self.regions[nb as usize].centre_cell(), goal_centre);
                    open.push_or_decrease(nb, tentative.saturating_add(h));
                }
            }
        }
        if !found {
            return None;
        }
        let mut path = vec![goal];
        let mut cur = goal;
        while cur != start {
            cur = parent[cur as usize];
            if cur == u32::MAX {
                return None;
            }
            path.push(cur);
        }
        path.reverse();
        Some(path)
    }
}

fn cell_of(px: i32) -> i32 {
    px.div_euclid(WALK_TILE_PX)
}

fn cell_center(c: i32) -> i32 {
    c.saturating_mul(WALK_TILE_PX)
        .saturating_add(WALK_TILE_PX / 2)
}

fn octile(a: (i32, i32), b: (i32, i32)) -> i32 {
    let dx = (a.0 - b.0).abs();
    let dy = (a.1 - b.1).abs();
    let (hi, lo) = if dx > dy { (dx, dy) } else { (dy, dx) };
    STRAIGHT_COST * (hi - lo) + DIAGONAL_COST * lo
}

fn cell_free(terrain: &Terrain, obstacles: &Obstacles, wx: i32, wy: i32, hw: i32, hh: i32) -> bool {
    let cx = cell_center(wx);
    let cy = cell_center(wy);
    let rect = Rect::from_center(cx, cy, hw, hh);
    if obstacles.blocks_rect(&rect) {
        return false;
    }
    let x0 = cell_of(rect.left);
    let y0 = cell_of(rect.top);
    let x1 = cell_of(rect.right - 1);
    let y1 = cell_of(rect.bottom - 1);
    for y in y0..=y1 {
        for x in x0..=x1 {
            if !terrain.walkable_cell(x, y) {
                return false;
            }
        }
    }
    true
}

/// Find the nearest free cell to `start`, spiralling outwards.  Mirrors the
/// original's behaviour when a unit's requested position is blocked.
fn nearest_free(
    terrain: &Terrain,
    obstacles: &Obstacles,
    start: (i32, i32),
    hw: i32,
    hh: i32,
    limit: i32,
) -> Option<(i32, i32)> {
    if cell_free(terrain, obstacles, start.0, start.1, hw, hh) {
        return Some(start);
    }
    for radius in 1..=limit {
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                if dx.abs() != radius && dy.abs() != radius {
                    continue;
                }
                let p = (start.0 + dx, start.1 + dy);
                if cell_free(terrain, obstacles, p.0, p.1, hw, hh) {
                    return Some(p);
                }
            }
        }
    }
    None
}

/// Line-of-sight smoothing using a dense walk-cell trace.  Returns true when
/// every cell centre along the segment is free for the footprint.
fn line_clear(
    terrain: &Terrain,
    obstacles: &Obstacles,
    a: (i32, i32),
    b: (i32, i32),
    hw: i32,
    hh: i32,
) -> bool {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let steps = dx.abs().max(dy.abs());
    if steps == 0 {
        return true;
    }
    for i in 0..=steps {
        let x = a.0 + dx * i / steps;
        let y = a.1 + dy * i / steps;
        if !cell_free(terrain, obstacles, x, y, hw, hh) {
            return false;
        }
    }
    true
}

fn simplify(
    terrain: &Terrain,
    obstacles: &Obstacles,
    cells: &[(i32, i32)],
    hw: i32,
    hh: i32,
) -> Vec<(i32, i32)> {
    if cells.len() <= 2 {
        return cells
            .iter()
            .map(|&(x, y)| (cell_center(x), cell_center(y)))
            .collect();
    }
    let mut out = Vec::new();
    let mut anchor = 0usize;
    while anchor + 1 < cells.len() {
        let mut far = cells.len() - 1;
        while far > anchor + 1 && !line_clear(terrain, obstacles, cells[anchor], cells[far], hw, hh)
        {
            far -= 1;
        }
        out.push((cell_center(cells[far].0), cell_center(cells[far].1)));
        anchor = far;
    }
    out
}

/// Low-level octile A* between two free walk cells.  Returns the full cell
/// chain (including both endpoints) and its cost.
///
/// The frontier uses the recovered `RetailOpen` heap (f only, newest-first on
/// ties) and the recovered neighbour visit order.  The heuristic remains the
/// exact `10*straight + 4*diagonal` octile bound; only the tie-break policy is
/// the one recovered from `0x7a305590`.
fn astar_cells(
    terrain: &Terrain,
    obstacles: &Obstacles,
    start: (i32, i32),
    goal: (i32, i32),
    hw: i32,
    hh: i32,
) -> Option<(Vec<(i32, i32)>, i32)> {
    if start == goal {
        return Some((vec![start], 0));
    }
    let w = terrain.walk_width as i32;
    let h = terrain.walk_height as i32;
    let n = (w * h) as usize;
    let mut g = vec![i32::MAX; n];
    let mut parent = vec![u32::MAX; n];
    let mut closed = vec![false; n];
    let index = |c: (i32, i32)| (c.1 * w + c.0) as usize;

    let mut open = RetailOpen::new(n);
    let si = index(start);
    g[si] = 0;
    open.push_or_decrease(si as u32, octile(start, goal));
    let mut expansions = 0usize;
    // A short trip to an enclosed goal (the free cell beside a mineral field,
    // walled in by the other fields and the town hall) flooded 120 000 cells
    // before failing; the budget grows with the distance instead (8000 + 400
    // per cell, at most 120 000), and a failed search falls back to a direct
    // move as before.
    let budget = (8_000 + 40 * octile(start, goal).max(0) as usize).min(120_000);
    let mut found = false;
    // Recovered `local_18` order from `FUN_7a304c10`.
    let dirs = [
        (-1, 0, STRAIGHT_COST),
        (0, -1, STRAIGHT_COST),
        (1, 0, STRAIGHT_COST),
        (0, 1, STRAIGHT_COST),
        (-1, -1, DIAGONAL_COST),
        (1, -1, DIAGONAL_COST),
        (-1, 1, DIAGONAL_COST),
        (1, 1, DIAGONAL_COST),
    ];

    while let Some(ci_u32) = open.pop_min() {
        let ci = ci_u32 as usize;
        if closed[ci] {
            continue;
        }
        closed[ci] = true;
        let cur = ((ci as i32) % w, (ci as i32) / w);
        if cur == goal {
            found = true;
            break;
        }
        expansions += 1;
        if expansions > budget {
            break;
        }
        for &(dx, dy, step) in &dirs {
            let nx = cur.0 + dx;
            let ny = cur.1 + dy;
            if nx < 0 || ny < 0 || nx >= w || ny >= h {
                continue;
            }
            if !cell_free(terrain, obstacles, nx, ny, hw, hh) {
                continue;
            }
            // The recovered A* forbids cutting a diagonal corner.
            if dx != 0
                && dy != 0
                && (!cell_free(terrain, obstacles, cur.0 + dx, cur.1, hw, hh)
                    || !cell_free(terrain, obstacles, cur.0, cur.1 + dy, hw, hh))
            {
                continue;
            }
            let ni = index((nx, ny));
            let ng = g[ci] + step;
            if ng < g[ni] {
                g[ni] = ng;
                parent[ni] = ci as u32;
                if closed[ni] {
                    // Recovered A* re-opens an improved closed node.
                    closed[ni] = false;
                }
                let f = ng + octile((nx, ny), goal);
                open.push_or_decrease(ni as u32, f);
            }
        }
    }

    if !found {
        return None;
    }
    let mut cells = Vec::new();
    let mut cur = goal;
    loop {
        cells.push(cur);
        let ci = index(cur);
        if cur == start {
            break;
        }
        let p = parent[ci];
        if p == u32::MAX {
            return None;
        }
        cur = ((p as i32) % w, (p as i32) / w);
    }
    cells.reverse();
    let cost = g[index(goal)];
    Some((cells, cost))
}

/// Compute a path.  Returns `None` when no route exists within the expansion
/// budget.  Flyers always receive a direct two-point path.
pub fn find_path(terrain: &Terrain, obstacles: &Obstacles, req: &PathRequest) -> Option<Path> {
    find_path_with_regions(terrain, obstacles, req, None)
}

pub fn find_path_with_regions(
    terrain: &Terrain,
    obstacles: &Obstacles,
    req: &PathRequest,
    regions: Option<&RegionMap>,
) -> Option<Path> {
    let goal = (cell_of(req.goal.0), cell_of(req.goal.1));
    if req.flying {
        let waypoints = if req.goal == req.start {
            Vec::new()
        } else {
            vec![req.goal]
        };
        let dx = req.goal.0 - req.start.0;
        let dy = req.goal.1 - req.start.1;
        let cost = ((dx * dx + dy * dy) as f64).sqrt() as i32;
        return Some(Path { waypoints, cost });
    }

    let start = (cell_of(req.start.0), cell_of(req.start.1));
    let start = nearest_free(
        terrain,
        obstacles,
        start,
        req.half_width,
        req.half_height,
        8,
    )?;
    let goal = nearest_free(
        terrain,
        obstacles,
        goal,
        req.half_width,
        req.half_height,
        12,
    )?;
    if let Some(regions) = regions
        && !regions.connected(start, goal)
    {
        return None;
    }
    if start == goal {
        return Some(Path {
            waypoints: if req.goal == req.start {
                Vec::new()
            } else {
                vec![(cell_center(goal.0), cell_center(goal.1))]
            },
            cost: 0,
        });
    }

    // Two-level pathfinder: first the recovered region graph, then the local
    // octile grid search.  The retail builder stores a bounding-box centre
    // for every region and links adjacent regions; long routes are therefore
    // routed through the intermediate region centres.  Short routes use the
    // grid search directly (the region route would only add a detour).
    let region_route = regions.and_then(|r| r.region_path(start, goal));
    if regions.is_some() && region_route.is_none() {
        return None;
    }
    let (cells, mut cost) = if let Some(route) = region_route {
        let mut combined: Option<(Vec<(i32, i32)>, i32)> = None;
        // Keep the coarse route bounded: running a full A* per region centre
        // is only worthwhile for a handful of regions.  Very long region
        // chains fall back to one grid search.
        if route.len() > 2 && route.len() <= 8 && octile(start, goal) > 200 {
            let mut points = vec![start];
            for rid in &route[1..route.len() - 1] {
                if let Some(r) = regions.and_then(|m| m.region(*rid)) {
                    let c = r.centre_cell();
                    let c = nearest_free(terrain, obstacles, c, req.half_width, req.half_height, 6)
                        .unwrap_or(c);
                    if points.last().copied() != Some(c) {
                        points.push(c);
                    }
                }
            }
            if *points.last().unwrap() != goal {
                points.push(goal);
            }
            let mut all: Vec<(i32, i32)> = Vec::new();
            let mut total = 0;
            let mut ok = true;
            for pair in points.windows(2) {
                if pair[0] == pair[1] {
                    continue;
                }
                match astar_cells(
                    terrain,
                    obstacles,
                    pair[0],
                    pair[1],
                    req.half_width,
                    req.half_height,
                ) {
                    Some((mut cs, c)) => {
                        if all.is_empty() {
                            all.append(&mut cs);
                        } else {
                            all.extend_from_slice(&cs[1..]);
                        }
                        total += c;
                    }
                    None => {
                        ok = false;
                        break;
                    }
                }
            }
            if ok {
                combined = Some((all, total));
            }
        }
        match combined {
            Some(v) => v,
            None => astar_cells(
                terrain,
                obstacles,
                start,
                goal,
                req.half_width,
                req.half_height,
            )?,
        }
    } else {
        astar_cells(
            terrain,
            obstacles,
            start,
            goal,
            req.half_width,
            req.half_height,
        )?
    };
    if cells.len() < 2 {
        return Some(Path {
            waypoints: Vec::new(),
            cost,
        });
    }
    let mut waypoints = simplify(
        terrain,
        obstacles,
        &cells[1..],
        req.half_width,
        req.half_height,
    );
    if goal == (cell_of(req.goal.0), cell_of(req.goal.1))
        && let Some(last) = waypoints.last_mut()
    {
        *last = req.goal;
    }
    if waypoints.is_empty() {
        waypoints.push(req.goal);
    }
    cost = cost.max(0);
    Some(Path { waypoints, cost })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_ground_takes_a_straight_line() {
        let terrain = Terrain::open(32, 32);
        let path = find_path(
            &terrain,
            &Obstacles::new(),
            &PathRequest {
                start: (16, 16),
                goal: (240, 16),
                half_width: 6,
                half_height: 6,
                flying: false,
            },
        )
        .unwrap();
        assert!(!path.waypoints.is_empty());
        assert_eq!(*path.waypoints.last().unwrap(), (240, 16));
        assert!(path.cost > 0);
    }

    #[test]
    fn wall_forces_a_detour() {
        let terrain = Terrain::open(32, 32);
        let mut obstacles = Obstacles::new();
        // A wall from y=0..20 tiles at x=16..17 tiles.
        obstacles.push(Rect::new(16 * 32, 0, 17 * 32, 20 * 32));
        let path = find_path(
            &terrain,
            &obstacles,
            &PathRequest {
                start: (8 * 32, 8 * 32),
                goal: (24 * 32, 8 * 32),
                half_width: 8,
                half_height: 8,
                flying: false,
            },
        )
        .unwrap();
        // The route must pass below the wall.
        assert!(path.waypoints.iter().any(|&(_, y)| y >= 20 * 32));
    }

    #[test]
    fn no_route_when_fully_walled() {
        let terrain = Terrain::open(16, 16);
        let mut obstacles = Obstacles::new();
        obstacles.push(Rect::new(8 * 32, 0, 9 * 32, 16 * 32));
        let path = find_path(
            &terrain,
            &obstacles,
            &PathRequest {
                start: (4 * 32, 8 * 32),
                goal: (12 * 32, 8 * 32),
                half_width: 8,
                half_height: 8,
                flying: false,
            },
        );
        assert!(path.is_none());
    }

    #[test]
    fn region_graph_uses_the_tile_grid_and_centres() {
        let mut terrain = Terrain::open(16, 16);
        let regions = RegionMap::from_terrain(&terrain);
        assert_eq!(regions.width, 16);
        assert_eq!(regions.height, 16);
        // `FUN_00763840`: regions are seeded from rectangles of at most 8 x 7
        // tiles and grown by one tile, so open ground is split (an open
        // 16 x 16 map is not one region).
        assert!(regions.count() >= 4, "{} regions", regions.count());
        let total: u32 = (0..regions.count())
            .filter_map(|i| regions.region(i))
            .map(|r| r.size)
            .sum();
        assert_eq!(total, 256, "every tile in exactly one region");
        let r = regions.region(0).unwrap();
        assert_eq!(r.min_tile, (0, 0));
        assert!(r.max_tile.0 - r.min_tile.0 < 10 && r.max_tile.1 - r.min_tile.1 < 9);
        assert!(regions.connected((0, 0), (60, 60)));

        // One 32-pixel (four walk-cell) column splits the map in two.
        for y in 0..64 {
            for x in 32..36 {
                terrain.set_walkable_cell(x, y, false);
            }
        }
        let regions = RegionMap::from_terrain(&terrain);
        assert!(!regions.connected((8, 8), (56, 56)));
        assert!(regions.region_path((8, 8), (56, 56)).is_none());

        // Open two minitile rows and the two regions join again.
        for x in 32..36 {
            terrain.set_walkable_cell(x, 30, true);
            terrain.set_walkable_cell(x, 31, true);
        }
        let regions = RegionMap::from_terrain(&terrain);
        assert!(regions.connected((8, 8), (56, 56)));
        assert!(regions.region_path((8, 8), (56, 56)).is_some());
    }

    #[test]
    fn region_neighbour_and_centre_lookups_match_the_recovered_helpers() {
        // `FUN_0075d6b0` returns the neighbour list (pointer + count) and
        // `FUN_0075d660` returns the region centre in pixels.
        let regions = RegionMap::from_terrain(&Terrain::open(16, 16));
        assert!(regions.count() >= 4);
        let id = 0;
        let neighbours = regions.neighbours_of(id);
        assert!(!neighbours.is_empty());
        assert!(!neighbours.contains(&id));
        for &nb in neighbours {
            assert!(nb < regions.count());
            assert!(
                regions.neighbours_of(nb).contains(&id),
                "adjacency is reciprocal"
            );
        }
        assert!(regions.neighbours_of(regions.count()).is_empty());

        let (cx, cy) = regions.region_centre_px(id).unwrap();
        let (cell_x, cell_y) = regions.region(id).unwrap().centre_cell();
        assert_eq!((cx, cy), (cell_x * 8, cell_y * 8));
        assert!((0..16 * 32).contains(&cx));
        assert!((0..16 * 32).contains(&cy));
        assert_eq!(regions.region_centre_px(regions.count()), None);
    }

    #[test]
    fn recovered_edge_masks_accept_orthogonal_neighbours_unconditionally() {
        // The recovered tile classifier accepts cases 0..=3 (the four
        // orthogonal neighbours) before consulting the walk-grid table.
        // Even two tiles whose only walkable minitiles are on opposite
        // corners connect.
        let mut terrain = Terrain::open(2, 1);
        for y in 0..4 {
            for x in 0..8 {
                terrain.set_walkable_cell(x, y, false);
            }
        }
        terrain.set_walkable_cell(0, 0, true);
        terrain.set_walkable_cell(7, 3, true);
        assert!(retail_tile_adjacent(&terrain, 0, 0, 1, 0));
        assert!(retail_tile_adjacent(&terrain, 1, 0, -1, 0));
        assert!(retail_tile_adjacent(&terrain, 0, 0, 0, 1));
    }

    #[test]
    fn recovered_edge_masks_require_corner_and_side_blocks_for_diagonals() {
        // Four tiles around a shared corner, all otherwise blocked.  Open
        // the 2x2 blocks that touch the corner in all four tiles.
        let mut terrain = Terrain::open(2, 2);
        for y in 0..8 {
            for x in 0..8 {
                terrain.set_walkable_cell(x, y, false);
            }
        }
        let open = |t: &mut Terrain, x0: i32, y0: i32| {
            for y in y0..y0 + 2 {
                for x in x0..x0 + 2 {
                    t.set_walkable_cell(x, y, true);
                }
            }
        };
        open(&mut terrain, 2, 2); // tile (0,0) bottom-right
        open(&mut terrain, 4, 4); // tile (1,1) top-left
        open(&mut terrain, 2, 4); // tile (0,1) top-right
        open(&mut terrain, 4, 2); // tile (1,0) bottom-left
        assert!(retail_tile_adjacent(&terrain, 1, 1, -1, -1));
        assert!(retail_tile_adjacent(&terrain, 0, 0, 1, 1));

        // Removing the whole horizontal side tile's corner still leaves the
        // vertical side route, which the recovered predicate accepts.
        for y in 4..6 {
            for x in 2..4 {
                terrain.set_walkable_cell(x, y, false);
            }
        }
        assert!(retail_tile_adjacent(&terrain, 1, 1, -1, -1));

        // With both side corners blocked the diagonal is rejected.
        for y in 2..4 {
            for x in 4..6 {
                terrain.set_walkable_cell(x, y, false);
            }
        }
        assert!(!retail_tile_adjacent(&terrain, 1, 1, -1, -1));

        // A missing cell in the diagonal tile's own corner also rejects it.
        open(&mut terrain, 2, 4);
        open(&mut terrain, 4, 2);
        terrain.set_walkable_cell(2, 2, false);
        assert!(!retail_tile_adjacent(&terrain, 1, 1, -1, -1));
    }

    #[test]
    fn flyers_ignore_obstacles() {
        let terrain = Terrain::open(16, 16);
        let mut obstacles = Obstacles::new();
        obstacles.push(Rect::new(0, 0, 16 * 32, 16 * 32));
        let path = find_path(
            &terrain,
            &obstacles,
            &PathRequest {
                start: (0, 0),
                goal: (400, 400),
                half_width: 8,
                half_height: 8,
                flying: true,
            },
        )
        .unwrap();
        assert_eq!(path.waypoints, vec![(400, 400)]);
    }

    #[test]
    fn retail_distance_matches_the_recovered_integer_formula() {
        // Values from the decompiled `FUN_7a1a4060` expression.
        for (delta, expected) in [
            ((0, 0), 0),
            ((1, 0), 1),
            ((10, 0), 10),
            ((10, 10), 13),
            ((5, 10), 11),
            ((10, 5), 11),
            ((3, 7), 8),
            ((100, 100), 131),
            ((100, 50), 111),
            ((255, 255), 334),
        ] {
            assert_eq!(retail_distance((0, 0), delta), expected, "delta {delta:?}");
            assert_eq!(
                retail_distance(delta, (0, 0)),
                expected,
                "reverse {delta:?}"
            );
        }
    }

    #[test]
    fn retail_open_list_orders_ties_by_the_recovered_heap_rules() {
        // `FUN_7a305e20` bubbles an equal-`f` entry above its parent, and
        // `FUN_7a305da0` keeps a parent that is <= its chosen child.  This
        // sequence is the observable consequence, including the fact that the
        // last-inserted equal-`f` entry (5) pops first.
        let mut open = RetailOpen::new(8);
        for (node, f) in [(0, 5), (1, 3), (2, 3), (3, 4), (4, 3), (5, 3)] {
            open.push_or_decrease(node, f);
        }
        let mut order = Vec::new();
        while let Some(node) = open.pop_min() {
            order.push(node);
        }
        assert_eq!(order, vec![5, 1, 2, 4, 3, 0]);
    }

    #[test]
    fn equal_cost_routes_use_the_recovered_neighbour_order() {
        // On an empty grid every monotone route has the same cost.  The
        // recovered visit order and heap tie-break must still produce a
        // stable, valid chain (the old implementation ordered ties by
        // `(f, g, x, y)`).
        let terrain = Terrain::open(32, 32);
        let obstacles = Obstacles::new();
        let (cells, _) = astar_cells(&terrain, &obstacles, (10, 10), (18, 18), 4, 4).unwrap();
        assert_eq!(cells.first(), Some(&(10, 10)));
        assert_eq!(cells.last(), Some(&(18, 18)));
        let step = (cells[1].0 - cells[0].0, cells[1].1 - cells[0].1);
        assert!(
            step.0.abs() <= 1 && step.1.abs() <= 1 && step != (0, 0),
            "first step must be an adjacent recovered cell, got {step:?}"
        );
        let (again, _) = astar_cells(&terrain, &obstacles, (10, 10), (18, 18), 4, 4).unwrap();
        assert_eq!(cells, again, "tie-breaking must be deterministic");
    }

    #[test]
    fn inclusive_point_test_matches_fun_00965210() {
        let r = Rect::new(10, 20, 30, 40);
        assert!(r.contains_point_inclusive(10, 20));
        assert!(r.contains_point_inclusive(30, 40));
        assert!(r.contains_point_inclusive(20, 30));
        assert!(!r.contains_point_inclusive(9, 20));
        assert!(!r.contains_point_inclusive(30, 41));
    }

    #[test]
    fn range_overlap_matches_fun_009308d0() {
        assert!(Rect::ranges_overlap(0, 5, 10));
        assert!(Rect::ranges_overlap(5, 0, 10));
        assert!(!Rect::ranges_overlap(0, 10, 10));
        assert!(!Rect::ranges_overlap(0, 11, 10));
    }
}

// ===========================================================================
// Merged from src/wave17/agent03.rs
// ===========================================================================
// Wave 17 translation batch 03: dialog hit-testing and small helpers.
//
// Widget layout (patch 1.23.10): rect of four `i16` at +4 (x1, y1, x2, y2),
// `u32` flags at +0x30, `i16` kind at +0x3c.  Opaque callees are closures.
// Paths that end in a non-returning call are modelled as `None`.

/// `0x00a8fb80`: builds a 4-dword record: copies the first three dwords of
/// `src` and stores `w` as the fourth.
pub fn w17a03_make_vec4(src: &[u32; 3], w: u32) -> [u32; 4] {
    [src[0], src[1], src[2], w]
}

/// `0x00a8fc10`: converts an RGB byte triple to a float RGBA colour
/// (each channel / 255.0, alpha = 1.0).
pub fn w17a03_rgb_bytes_to_float(rgb: &[u8; 3]) -> [f32; 4] {
    [
        rgb[0] as f32 / 255.0,
        rgb[1] as f32 / 255.0,
        rgb[2] as f32 / 255.0,
        1.0,
    ]
}

/// `0x00754880`: picks a result code.  `a` true -> 6; else if `b` true ->
/// 0x55 (or 6 when `d`); else if `c` true -> 0x51 (or 6 when `d`); else the
/// original does not return (`None`).
pub fn w17a03_select_code(a: bool, b: bool, c: bool, d: impl FnOnce() -> bool) -> Option<u32> {
    if a {
        return Some(6);
    }
    if b {
        let m = if d() { 0xffff_ffb1u32 } else { 0 };
        return Some(m.wrapping_add(0x55));
    }
    if c {
        let m = if d() { 0xffff_ffb5u32 } else { 0 };
        return Some(m.wrapping_add(0x51));
    }
    None
}

/// `0x0065134c`: `Some(0)` when `kind == 0x23` or (`p1` and `p2`); `Some(1)`
/// when `p3` and `p4`; otherwise the original does not return (`None`).
pub fn w17a03_kind_gate(
    kind: i16,
    p1: impl FnOnce() -> bool,
    p2: impl FnOnce() -> bool,
    p3: impl FnOnce() -> bool,
    p4: impl FnOnce() -> bool,
) -> Option<u32> {
    if kind == 0x23 {
        return Some(0);
    }
    if p1() && p2() {
        return Some(0);
    }
    if p3() && p4() { Some(1) } else { None }
}

/// `0x00768080`: `None` (non-returning path) when `cond` holds and the byte
/// at +0xd8 is zero, else `Some(0)`.
pub fn w17a03_flag_gate(cond: bool, byte_d8: u8) -> Option<u32> {
    if cond && byte_d8 == 0 { None } else { Some(0) }
}

/// `0x00782440`: if the widget is active and the mapped point lies in its
/// rect, returns `Some(f(px - x1, x2 - x1))` (`f` = retail 0x75_c1a0).
pub fn w17a03_point_offset(
    rect: &[i16; 4],
    active: bool,
    mut map_point: impl FnMut() -> (i16, i16),
    mut contains: impl FnMut(&[i16; 4], &mut (i16, i16)) -> bool,
    f: impl FnOnce(i32, i32) -> u32,
) -> Option<u32> {
    if !active {
        return None;
    }
    let mut pt = map_point();
    if !contains(rect, &mut pt) {
        return None;
    }
    let x1 = rect[0] as i32;
    Some(f(pt.0 as i32 - x1, rect[2] as i32 - x1))
}

/// `0x0097d0e0`: hover toggle on flag bit 0x40000000.  Leaving clears it;
/// entering calls `on_enter` then sets it.  Returns the new flags.
pub fn w17a03_hover_toggle(flags: u32, inside: bool, on_enter: impl FnOnce()) -> u32 {
    let had = flags & 0x4000_0000 != 0;
    if !inside {
        if had {
            return flags ^ 0x4000_0000;
        }
    } else if !had {
        on_enter();
        return flags ^ 0x4000_0000;
    }
    flags
}

/// Constants consumed by `w17a03_scroll_hit` (globals 0x011cdd74.. in retail).
#[derive(Clone, Copy, Debug, Default)]
pub struct W17a03ScrollMetrics {
    /// Retail 0x011cddcc (default tail length).
    pub tail_default: u16,
    /// Retail 0x011cdd78 / 0x011cdd88 (horizontal pad, tail).
    pub h_pad: u16,
    pub h_tail: u16,
    /// Retail 0x011cdd74 / 0x011cdd8c (vertical pad, tail).
    pub v_pad: u16,
    pub v_tail: u16,
}

/// `0x00982d80`: classifies a point against a scrollbar widget.  Returns 0
/// outside the rect, 6 when outside the inner rect, 1/2 for the end caps,
/// 4 before the thumb, 3 on the thumb, 5 after it.  `thumb_len` is retail
/// 0x98_2610.
pub fn w17a03_scroll_hit(
    rect: &[i16; 4],
    flags: u32,
    kind: i16,
    m: &W17a03ScrollMetrics,
    thumb_len: i16,
    mut pt: (i16, i16),
    mut contains: impl FnMut(&[i16; 4], &mut (i16, i16)) -> bool,
) -> u8 {
    if !contains(rect, &mut pt) {
        return 0;
    }
    let inner = *rect;
    if !contains(&inner, &mut pt) {
        return 6;
    }
    let no_flag = flags & 0x400_0000 == 0;
    let mut tail = m.tail_default;
    let (extent, pos, origin, pad);
    if kind == 6 {
        extent = rect[2].wrapping_sub(rect[0]);
        pos = pt.0;
        origin = rect[0];
        pad = if no_flag {
            tail = m.h_tail;
            m.h_pad as u32
        } else {
            0
        };
    } else {
        extent = rect[3].wrapping_sub(rect[1]);
        pos = pt.1;
        origin = rect[1];
        pad = if no_flag {
            tail = m.v_tail;
            m.v_pad as u32
        } else {
            0
        };
    }
    let pad = pad + 2;
    let pad16 = (pad & 0xffff) as i32;
    if (pos as i32) <= origin as i32 + pad16 {
        return 1;
    }
    if (extent.wrapping_add(1) as i32 - pad16) + origin as i32 <= pos as i32 {
        return 2;
    }
    let t = origin
        .wrapping_add(thumb_len)
        .wrapping_add(pad as u16 as i16);
    if pos < t {
        return 4;
    }
    (((tail as i16).wrapping_add(t) <= pos) as u8) * 2 + 3
}

#[cfg(test)]
mod tests_wave17_agent03 {
    use super::*;

    #[test]
    fn vec_and_colour() {
        assert_eq!(w17a03_make_vec4(&[1, 2, 3], 9), [1, 2, 3, 9]);
        let c = w17a03_rgb_bytes_to_float(&[255, 0, 51]);
        assert_eq!(c, [1.0, 0.0, 0.2, 1.0]);
    }

    #[test]
    fn select_code() {
        assert_eq!(w17a03_select_code(true, false, false, || true), Some(6));
        assert_eq!(w17a03_select_code(false, true, false, || false), Some(0x55));
        assert_eq!(w17a03_select_code(false, true, false, || true), Some(6));
        assert_eq!(w17a03_select_code(false, false, true, || false), Some(0x51));
        assert_eq!(w17a03_select_code(false, false, true, || true), Some(6));
        assert_eq!(w17a03_select_code(false, false, false, || true), None);
    }

    #[test]
    fn gates() {
        let t = || true;
        let f = || false;
        assert_eq!(w17a03_kind_gate(0x23, f, f, f, f), Some(0));
        assert_eq!(w17a03_kind_gate(1, t, t, f, f), Some(0));
        assert_eq!(w17a03_kind_gate(1, f, f, t, t), Some(1));
        assert_eq!(w17a03_kind_gate(1, f, f, t, f), None);
        assert_eq!(w17a03_flag_gate(true, 0), None);
        assert_eq!(w17a03_flag_gate(true, 1), Some(0));
        assert_eq!(w17a03_flag_gate(false, 0), Some(0));
    }

    #[test]
    fn offset_and_hover() {
        let r = [10, 0, 50, 5];
        let inside = |_: &[i16; 4], _: &mut (i16, i16)| true;
        assert_eq!(
            w17a03_point_offset(&r, true, || (20, 1), inside, |a, b| (a * 100 + b) as u32),
            Some(10 * 100 + 40)
        );
        assert_eq!(
            w17a03_point_offset(&r, false, || (20, 1), inside, |_, _| 0),
            None
        );
        let mut n = 0;
        assert_eq!(w17a03_hover_toggle(0, true, || n += 1), 0x4000_0000);
        assert_eq!(n, 1);
        assert_eq!(
            w17a03_hover_toggle(0x4000_0001, true, || n += 1),
            0x4000_0001
        );
        assert_eq!(w17a03_hover_toggle(0x4000_0001, false, || {}), 1);
        assert_eq!(w17a03_hover_toggle(1, false, || {}), 1);
    }

    #[test]
    fn scroll() {
        let r = [0, 0, 100, 10];
        let m = W17a03ScrollMetrics {
            tail_default: 8,
            h_pad: 4,
            h_tail: 6,
            v_pad: 0,
            v_tail: 0,
        };
        let c = |_: &[i16; 4], _: &mut (i16, i16)| true;
        let hit = |x| w17a03_scroll_hit(&r, 0, 6, &m, 10, (x, 5), c);
        assert_eq!(hit(5), 1);
        assert_eq!(hit(95), 2);
        assert_eq!(hit(10), 4); // thumb starts at 0+10+6=16
        assert_eq!(hit(18), 3);
        assert_eq!(hit(30), 5);
        let no = |_: &[i16; 4], _: &mut (i16, i16)| false;
        assert_eq!(w17a03_scroll_hit(&r, 0, 6, &m, 10, (5, 5), no), 0);
    }
}

// ===========================================================================
// Merged from src/wave18/agent02.rs
// ===========================================================================
// Wave 18 translation batch 02: player-colour lookup, HUD colour slots,
// rect geometry and a 64-bit signed divide helper.
//
// Colour values are 16-byte `[u32; 4]` records (opaque float vectors);
// palette lookups and colour maths of unresolved callees are closures.

/// Context for the player colour lookup (state blob is the XOR-decoded
/// game-state pointer base, patch 1.23.10).
#[allow(non_camel_case_types)]
pub struct w18a02_ColorCtx<'a> {
    /// `DAT_0117ee68 == 2` (replay/observer-style branch).
    pub mode2: bool,
    /// Override table enabled flag (`DAT_00fa3588 != 0`).
    pub override_on: bool,
    /// Override colours, 8 entries of 16 bytes.
    pub override_colors: &'a [[u32; 4]],
    /// Local player slot (`DAT_00f2acf4`).
    pub local_player: u32,
    /// State blob: relation bytes at `(local*3+0x3951)*4 + slot`, u32 table
    /// at `300..0x14c`, palette-index bytes at `0x2c86 + slot*8`.
    pub blob: &'a [u8],
}

fn w18a02_blob_u32(blob: &[u8], off: usize) -> Option<u32> {
    let b = blob.get(off..off + 4)?;
    Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// `0x005975c0`: colour of player `slot`.  Outside mode 2: the override
/// table when enabled and `slot < 8`, else palette[blob[0x2c86+slot*8]].
/// In mode 2 the slot is first remapped (2 if self, 7 if related else 0)
/// and searched in the u32 table `300..0x14c` (8 entries); the found index
/// (or the searched value when absent) selects the palette byte.  Returns
/// `None` when `noreturn` (callee 0x74_8d80 true) or blob is too short.
pub fn w18a02_player_color(
    ctx: &w18a02_ColorCtx,
    slot: u8,
    noreturn: bool,
    palette: &dyn Fn(u8) -> [u32; 4],
) -> Option<[u32; 4]> {
    let pal_byte = |s: u32| ctx.blob.get(0x2c86 + (s as usize & 0xff) * 8).copied();
    if !ctx.mode2 {
        if ctx.override_on && slot < 8 {
            return ctx.override_colors.get(slot as usize).copied();
        }
        return Some(palette(pal_byte(slot as u32)?));
    }
    if noreturn {
        return None;
    }
    let s = slot as u32;
    let want = if ctx.local_player == s {
        2
    } else {
        let off = (ctx.local_player as usize * 3 + 0x3951) * 4 + s as usize;
        if *ctx.blob.get(off)? == 0 { 0 } else { 7 }
    };
    let mut found = want;
    let mut off = 300usize;
    let mut i = 0u32;
    while off < 0x14c {
        if w18a02_blob_u32(ctx.blob, off)? == want {
            found = i;
            break;
        }
        off += 4;
        i += 1;
    }
    Some(palette(pal_byte(found)?))
}

/// `0x00597a30`: same as `0x005975c0` with the slot taken from the local
/// player (`DAT_00f2acf4`, low byte).
pub fn w18a02_local_player_color(
    ctx: &w18a02_ColorCtx,
    noreturn: bool,
    palette: &dyn Fn(u8) -> [u32; 4],
) -> Option<[u32; 4]> {
    w18a02_player_color(ctx, ctx.local_player as u8, noreturn, palette)
}

/// `0x006ffa40`: maps a HUD colour index 0..=9 to a palette index using the
/// six bytes stored at 0x117a418, 41b, 41e, 421, 424, 427 (in that order,
/// `idx`).  Out of range returns `None` (caller keeps the default colour).
pub fn w18a02_hud_palette_index(idx: &[u8; 6], which: u32) -> Option<u8> {
    let k = match which {
        0 | 5 => 2,
        1 | 6 => 1,
        2 | 7 => 0,
        3 => 4,
        4 | 9 => 5,
        8 => 3,
        _ => return None,
    };
    Some(idx[k])
}

/// `0x0075c1a0`: scales `value` against `total` into `0..=cap`:
/// `total == 0` gives `fallback`; else `min(cap*value/total, cap)`
/// (unsigned 32-bit division, as in the retail code).
pub fn w18a02_scaled_cap(value: u32, total: u32, cap: u32, fallback: u32) -> u32 {
    if total == 0 {
        return fallback;
    }
    let scaled = cap.wrapping_mul(value) / total;
    scaled.min(cap)
}

/// `0x00957720`: colour record for a status kind: palette entry selected by
/// byte `+1` of the 8-byte row for `kind` (row supplied by the caller from
/// `FUN_00957300`), plus a flag set when `kind == 5`.
pub fn w18a02_status_color(
    kind: i32,
    row_palette_byte: u8,
    palette: &dyn Fn(u8) -> [u32; 4],
) -> ([u32; 4], bool) {
    (palette(row_palette_byte), kind == 5)
}

/// Two-deep colour slot (current/previous) with a flag byte.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub struct w18a02_ColorSlots {
    pub cur: [u32; 4],
    pub cur_flag: u8,
    pub prev: [u32; 4],
    pub prev_flag: u8,
}

/// `0x009583e0`: `kind == 1` restores current from previous; otherwise
/// saves current into previous and loads `palette_color` (looked up by the
/// caller) with flag `kind == 5`.
pub fn w18a02_push_color(s: &mut w18a02_ColorSlots, kind: i8, palette_color: [u32; 4]) {
    if kind == 1 {
        s.cur = s.prev;
        s.cur_flag = s.prev_flag;
        return;
    }
    s.prev = s.cur;
    s.prev_flag = s.cur_flag;
    s.cur = palette_color;
    s.cur_flag = (kind == 5) as u8;
}

/// `0x00958470`: clamps `t` to `[0,1]` (NaN gives 0), builds the blended
/// colour via `blend(0xac, 0xf4, 0x6c, t)` and pushes it with flag 0.
/// Returns the clamped `t`.
pub fn w18a02_push_blend(
    s: &mut w18a02_ColorSlots,
    t: f32,
    blend: &dyn Fn(u8, u8, u8, f32) -> [u32; 4],
) -> f32 {
    let t = if 0.0 <= t {
        if t <= 1.0 { t } else { 1.0 }
    } else {
        0.0
    };
    let color = blend(0xac, 0xf4, 0x6c, t);
    s.prev = s.cur;
    s.prev_flag = s.cur_flag;
    s.cur = color;
    s.cur_flag = 0;
    t
}

/// Output of `0x006995e0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub struct w18a02_BarRects {
    /// `*param_3`: (x0, y0, x1, y0 + 600).
    pub out3: [i16; 4],
    /// `*param_4` and the rect drawn: (x0, y2, x1, y3).
    pub out4: [i16; 4],
    /// Width written to the 0x11cb5ec record (`right-left+1`).
    pub width: u16,
    /// Constant stored beside it.
    pub height_const: u16,
}

/// `0x006995e0`: bar geometry.  `rect` is the widget (left, top, right,
/// bottom) = i16 at +4/+6/+8/+10, `y_off` the i16 at +0x3e, `pos` the
/// (x, y) origin.  All arithmetic wraps in 16 bits.
pub fn w18a02_bar_rects(rect: [i16; 4], y_off: i16, pos: [i16; 2]) -> w18a02_BarRects {
    let [left, top, right, bottom] = rect;
    let x0 = pos[0].wrapping_add(left);
    let y0 = pos[1].wrapping_add(top.wrapping_sub(y_off));
    let width = right.wrapping_sub(left).wrapping_add(1) as u16;
    let x1 = (width as i16).wrapping_add(x0);
    let y2 = pos[1].wrapping_add(top);
    let y3 = bottom.wrapping_sub(top).wrapping_add(1).wrapping_add(y2);
    w18a02_BarRects {
        out3: [x0, y0, x1, y0.wrapping_add(600)],
        out4: [x0, y2, x1, y3],
        width,
        height_const: 700,
    }
}

/// `0x00c87550`: signed 64-bit divide (CRT `__alldiv`): quotient truncated
/// toward zero; `None` on zero divisor (hardware fault in retail).
pub fn w18a02_alldiv(num: i64, den: i64) -> Option<i64> {
    if den == 0 {
        return None;
    }
    let neg = (num < 0) != (den < 0);
    let q = num.unsigned_abs() / den.unsigned_abs();
    Some(if neg {
        (q as i64).wrapping_neg()
    } else {
        q as i64
    })
}

#[cfg(test)]
mod tests_wave18_agent02 {
    use super::*;

    fn pal(i: u8) -> [u32; 4] {
        [i as u32, 0, 0, 1]
    }

    #[test]
    fn hud_palette_index_map() {
        let t = [10, 11, 12, 13, 14, 15];
        assert_eq!(w18a02_hud_palette_index(&t, 0), Some(12));
        assert_eq!(w18a02_hud_palette_index(&t, 5), Some(12));
        assert_eq!(w18a02_hud_palette_index(&t, 6), Some(11));
        assert_eq!(w18a02_hud_palette_index(&t, 7), Some(10));
        assert_eq!(w18a02_hud_palette_index(&t, 3), Some(14));
        assert_eq!(w18a02_hud_palette_index(&t, 9), Some(15));
        assert_eq!(w18a02_hud_palette_index(&t, 8), Some(13));
        assert_eq!(w18a02_hud_palette_index(&t, 10), None);
    }

    #[test]
    fn scaled_cap_bounds() {
        assert_eq!(w18a02_scaled_cap(5, 0, 100, 77), 77);
        assert_eq!(w18a02_scaled_cap(5, 10, 100, 0), 50);
        assert_eq!(w18a02_scaled_cap(30, 10, 100, 0), 100);
    }

    #[test]
    fn player_color_normal_and_override() {
        let mut blob = vec![0u8; 0x4000];
        blob[0x2c86 + 3 * 8] = 9;
        let ov = [[7u32; 4]; 8];
        let mut c = w18a02_ColorCtx {
            mode2: false,
            override_on: false,
            override_colors: &ov,
            local_player: 1,
            blob: &blob,
        };
        assert_eq!(w18a02_player_color(&c, 3, false, &pal), Some(pal(9)));
        c.override_on = true;
        assert_eq!(w18a02_player_color(&c, 3, false, &pal), Some([7; 4]));
        assert_eq!(w18a02_player_color(&c, 8, false, &pal), Some(pal(0)));
        assert_eq!(w18a02_local_player_color(&c, false, &pal), Some([7; 4]));
    }

    #[test]
    fn player_color_mode2() {
        let mut blob = vec![0u8; 0x10000];
        // table entries 300.. : index 3 holds value 2
        blob[300 + 12] = 2;
        blob[0x2c86 + 3 * 8] = 42;
        // slot absent: want 0 -> found = 0 (table[0]==0 matches first)
        blob[0x2c86] = 5;
        let ov = [[0u32; 4]; 8];
        let c = w18a02_ColorCtx {
            mode2: true,
            override_on: false,
            override_colors: &ov,
            local_player: 4,
            blob: &blob,
        };
        // self slot: want 2, found at index 3 (table 0..2 are zero)
        assert_eq!(w18a02_player_color(&c, 4, false, &pal), Some(pal(42)));
        // unrelated slot: want 0, first entry matches
        assert_eq!(w18a02_player_color(&c, 1, false, &pal), Some(pal(5)));
        assert_eq!(w18a02_player_color(&c, 1, true, &pal), None);
    }

    #[test]
    fn status_color_flag() {
        assert_eq!(w18a02_status_color(5, 3, &pal), (pal(3), true));
        assert_eq!(w18a02_status_color(4, 3, &pal), (pal(3), false));
    }

    #[test]
    fn color_slot_push_and_restore() {
        let mut s = w18a02_ColorSlots::default();
        w18a02_push_color(&mut s, 5, [1; 4]);
        assert_eq!((s.cur, s.cur_flag), ([1; 4], 1));
        w18a02_push_color(&mut s, 2, [2; 4]);
        assert_eq!((s.cur, s.cur_flag, s.prev), ([2; 4], 0, [1; 4]));
        w18a02_push_color(&mut s, 1, [9; 4]);
        assert_eq!((s.cur, s.cur_flag), ([1; 4], 1));
    }

    #[test]
    fn push_blend_clamps() {
        let mut s = w18a02_ColorSlots::default();
        let b = |r: u8, g: u8, bl: u8, t: f32| [r as u32, g as u32, bl as u32, t.to_bits()];
        assert_eq!(w18a02_push_blend(&mut s, 2.0, &b), 1.0);
        assert_eq!(s.cur[0], 0xac);
        assert_eq!(w18a02_push_blend(&mut s, -1.0, &b), 0.0);
        assert_eq!(w18a02_push_blend(&mut s, f32::NAN, &b), 0.0);
        assert_eq!(w18a02_push_blend(&mut s, 0.5, &b), 0.5);
        assert_eq!(s.cur_flag, 0);
    }

    #[test]
    fn bar_rects_geometry() {
        let r = w18a02_bar_rects([10, 20, 19, 29], 5, [100, 200]);
        assert_eq!(r.out3, [110, 215, 120, 815]);
        assert_eq!(r.out4, [110, 220, 120, 230]);
        assert_eq!((r.width, r.height_const), (10, 700));
    }

    #[test]
    fn alldiv_matches_i64() {
        for (a, b) in [
            (7i64, 2i64),
            (-7, 2),
            (7, -2),
            (-7, -2),
            (i64::MIN, 3),
            (1 << 40, 1 << 33),
            (5, 1 << 35),
        ] {
            assert_eq!(w18a02_alldiv(a, b), Some(a / b));
        }
        assert_eq!(w18a02_alldiv(1, 0), None);
    }
}

// ===========================================================================
// Merged from src/wave18/agent03.rs
// ===========================================================================
// Wave 18 translation batch 03: weak-ref lock, scrollbar/widget geometry.

/// `0x00535260`: shared-pointer "lock" (weak -> strong).  Copies the
/// `(ptr, control)` pair; if a control block exists, atomically bumps its use
/// count (`+4`) unless it is zero, in which case the pointer is cleared.
/// Returns the new `(ptr, control_present)` pair and updates `use_count`.
pub fn w18a03_weak_lock(
    ptr: u32,
    has_control: bool,
    use_count: &std::sync::atomic::AtomicI32,
) -> (u32, bool) {
    use std::sync::atomic::Ordering;
    if !has_control {
        return (ptr, false);
    }
    let mut cur = use_count.load(Ordering::SeqCst);
    loop {
        if cur == 0 {
            return (0, true);
        }
        match use_count.compare_exchange(cur, cur + 1, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => return (ptr, true),
            Err(actual) => cur = actual,
        }
    }
}

/// `0x009646c0`: packs widget flag bits at `+0x30`: bit 0 = flag bit 1,
/// bits 8.. = flags >> 9 (bit 1 of the low byte and bits 1..8 cleared).
pub fn w18a03_flag_bits_1_and_9(flags: u32) -> u32 {
    (((flags >> 9) << 8) | ((flags >> 1) & 0xff)) & 0xffff_ff01
}

/// `0x009651b0`: offset of event position (`+0x12`, `+0x14`) from the
/// widget origin (`+4`, `+6`); when `+0x3c != 0` the origin comes from the
/// parent widget (`+0x4c`), passed as `parent_origin`.
pub fn w18a03_event_offset(
    kind3c: i16,
    own_origin: [i16; 2],
    parent_origin: [i16; 2],
    event_pos: [i16; 2],
) -> [i16; 2] {
    let o = if kind3c != 0 {
        parent_origin
    } else {
        own_origin
    };
    [
        event_pos[0].wrapping_sub(o[0]),
        event_pos[1].wrapping_sub(o[1]),
    ]
}

/// `0x0097b550`: rectangle geometry part: `rect = [l, t, r, b]` (i16)
/// offset by `off = [x, y]`; returns `(left, top, right, bottom)` as the
/// call order of the draw helper (x offset on columns 0/2, y on 1/3).
pub fn w18a03_offset_rect(rect: [i16; 4], off: [i16; 2]) -> (i32, i32, i32, i32) {
    (
        rect[0] as i32 + off[0] as i32,
        rect[1] as i32 + off[1] as i32,
        rect[2] as i32 + off[0] as i32,
        rect[3] as i32 + off[1] as i32,
    )
}

/// Scrollbar fields used by the two routines below (offsets in comments).
#[derive(Clone, Copy, Debug, Default)]
pub struct W18a03Scroll {
    /// `+0x30`
    pub flags: u32,
    /// `+0x64`
    pub thumb: u16,
    /// `+0x54`
    pub cur: i16,
    /// `+0x56`
    pub min: i16,
    /// `+0x58`
    pub max: i16,
    /// `+0x5a`
    pub value: i16,
    /// `+0x5c`
    pub mask: u32,
    /// `+0x3c`
    pub kind: i16,
    /// `+4..+0xc` rect (l, t, r, b)
    pub rect: [i16; 4],
}

/// `0x00982610`: scrollbar position.  `track_len` is the result of the
/// length helper (callee `FUN_00982690`).  Negative flags return the stored
/// thumb; empty range returns 0; ends return exact values; else rounds.
pub fn w18a03_scroll_position(s: &W18a03Scroll, track_len: i16) -> u32 {
    if (s.flags as i32) < 0 {
        return s.thumb as u32;
    }
    let span = s.max.wrapping_sub(s.min);
    if span != 0 && s.cur != s.min {
        if s.cur == s.max {
            return track_len as u16 as u32;
        }
        let num = span as i32 / 2 + (s.cur as i32 - s.min as i32) * track_len as i32;
        return (num / span as i32) as u32;
    }
    0
}

/// Layout globals read by `0x00982720` (DAT values near `11cdd70`).
#[derive(Clone, Copy, Debug, Default)]
pub struct W18a03Metrics {
    pub d70: i32,
    pub d74: i32,
    pub d78: i32,
    pub d88: i32,
    pub d8c: i32,
    pub dcc: i32,
    pub dd0: i32,
}

/// Follow-up action of `0x00982720` for the non-drag path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W18a03Follow {
    None,
    /// indirect (guarded) callback invoked
    Callback,
    /// notify helper `FUN_0077ed00` invoked
    Notify,
}

/// `0x00982720`: scrollbar mouse handler.  `idx_table` is the `d7afa8`
/// table indexed by the result of `FUN_00982d80`; `offset` is the event
/// offset (see `w18a03_event_offset`); `len_fn` is `FUN_00982690`.
/// Returns the follow-up action.
pub fn w18a03_scroll_event(
    s: &mut W18a03Scroll,
    idx_table: &[u32],
    idx: usize,
    offset: [i16; 2],
    m: &W18a03Metrics,
    len_fn: impl Fn(&W18a03Scroll) -> i16,
) -> W18a03Follow {
    if s.mask == 0x8000_0000 {
        let sticky = s.flags & 0x0400_0000;
        let (pos, mut size, size_alt, pad);
        if s.kind == 6 {
            let mut lim = if sticky == 0 {
                (s.rect[2] as i32 - (m.d88 >> 1) - m.d70 - 3) as i16
            } else {
                (s.rect[2] as i32 - (m.dcc >> 1)) as i16
            };
            if offset[0] < lim {
                lim = offset[0];
            }
            pos = lim as i32 - s.rect[0] as i32;
            pad = if sticky == 0 { m.d78 } else { 0 };
            size = m.d88;
            size_alt = m.dcc;
        } else {
            let mut lim = if sticky == 0 {
                (s.rect[3] as i32 - (m.d8c >> 1) - m.d74 - 3) as i16
            } else {
                (s.rect[2] as i32 - (m.dd0 >> 1)) as i16
            };
            if offset[1] < lim {
                lim = offset[1];
            }
            pos = lim as i32 - s.rect[1] as i32;
            pad = if sticky == 0 { m.d74 + 2 } else { 0 };
            size = m.d8c;
            size_alt = m.dd0;
        }
        if sticky != 0 {
            size = size_alt;
        }
        let v = pos
            .wrapping_sub(((size as u32) >> 1) as i32)
            .wrapping_sub(pad);
        let v = v.max(0);
        let min = s.min;
        s.thumb = v as u16;
        let l = len_fn(s) as i32;
        let num = (s.max as i32 - min as i32 + 1) * v;
        let q = if l == 0 { 0 } else { num / l };
        s.value = (q as i16).wrapping_sub(s.cur).wrapping_add(min);
        return W18a03Follow::None;
    }
    let mask = s.mask;
    let tbl = idx_table.get(idx).copied().unwrap_or(0);
    if mask & s.flags == 0 {
        if tbl == mask {
            s.flags ^= mask;
            if s.mask & s.flags != 0 {
                return W18a03Follow::Notify;
            }
            return W18a03Follow::Callback;
        }
    } else if tbl != mask {
        s.flags ^= mask;
        return W18a03Follow::Callback;
    }
    W18a03Follow::None
}

#[cfg(test)]
mod tests_wave18_agent03 {
    use super::*;
    use std::sync::atomic::AtomicI32;

    #[test]
    fn weak_lock() {
        let c = AtomicI32::new(2);
        assert_eq!(w18a03_weak_lock(7, true, &c), (7, true));
        assert_eq!(c.load(std::sync::atomic::Ordering::SeqCst), 3);
        let z = AtomicI32::new(0);
        assert_eq!(w18a03_weak_lock(7, true, &z), (0, true));
        assert_eq!(w18a03_weak_lock(7, false, &z), (7, false));
    }

    #[test]
    fn flags_and_offsets() {
        assert_eq!(w18a03_flag_bits_1_and_9(0x2), 0x1);
        assert_eq!(w18a03_flag_bits_1_and_9(0x200) & 0xffff_ff01, 0x100);
        assert_eq!(w18a03_event_offset(0, [1, 2], [10, 20], [5, 5]), [4, 3]);
        assert_eq!(w18a03_event_offset(1, [1, 2], [10, 20], [5, 5]), [-5, -15]);
        assert_eq!(w18a03_offset_rect([1, 2, 3, 4], [10, 20]), (11, 22, 13, 24));
    }

    #[test]
    fn position() {
        let mut s = W18a03Scroll {
            min: 0,
            max: 10,
            cur: 0,
            ..Default::default()
        };
        assert_eq!(w18a03_scroll_position(&s, 100), 0);
        s.cur = 10;
        assert_eq!(w18a03_scroll_position(&s, 100), 100);
        s.cur = 5;
        assert_eq!(w18a03_scroll_position(&s, 100), 50);
        s.flags = 0x8000_0000;
        s.thumb = 9;
        assert_eq!(w18a03_scroll_position(&s, 100), 9);
    }

    #[test]
    fn event_drag_and_toggle() {
        let m = W18a03Metrics {
            d88: 10,
            dcc: 10,
            d8c: 10,
            dd0: 10,
            ..Default::default()
        };
        let mut s = W18a03Scroll {
            mask: 0x8000_0000,
            kind: 6,
            rect: [0, 0, 200, 100],
            min: 0,
            max: 9,
            ..Default::default()
        };
        let r = w18a03_scroll_event(&mut s, &[], 0, [50, 0], &m, |_| 10);
        assert_eq!(r, W18a03Follow::None);
        assert_eq!(s.thumb, 45);
        assert_eq!(s.value, 450 / 10);
        let mut t = W18a03Scroll {
            mask: 4,
            ..Default::default()
        };
        assert_eq!(
            w18a03_scroll_event(&mut t, &[4], 0, [0, 0], &m, |_| 1),
            W18a03Follow::Notify
        );
        assert_eq!(t.flags, 4);
        assert_eq!(
            w18a03_scroll_event(&mut t, &[0], 0, [0, 0], &m, |_| 1),
            W18a03Follow::Callback
        );
        assert_eq!(t.flags, 0);
    }
}

// ===========================================================================
// Merged from src/wave19/agent02.rs
// ===========================================================================
// Wave 19 translation batch 02: small pure helpers (path.rs target).

/// `0x00748d80`: returns `false` only when the flag word is zero and
/// `code` is outside `0x80..=0x83`, otherwise `true`.
pub fn w19a02_code_or_flag_allowed(flag: u32, code: u32) -> bool {
    !(flag == 0 && code.wrapping_sub(0x80) > 3)
}

/// `0x00748ee0`: fetches a 16-byte record (four words) for `code`.  When
/// `mode_two` (global byte == 2) the code `0x80..=0x83` is remapped to
/// `0x0c..=0x0f`, the byte at `code * 0x24 + 10` (read through `row_byte`)
/// minus one selects the 8-entry table row (negative -> 7).  Otherwise
/// `fallback(code)` supplies the record.
pub fn w19a02_record_for_code(
    mode_two: bool,
    code: u32,
    row_byte: impl Fn(u32) -> u8,
    table: &[[u32; 4]; 8],
    fallback: impl FnOnce(u32) -> [u32; 4],
) -> [u32; 4] {
    if !mode_two {
        return fallback(code);
    }
    let mut code = code;
    if (0x80..0x84).contains(&code) {
        code = (code & 0xffff_ff7f) + 0xc;
    }
    let mut idx = i32::from(row_byte(code)) - 1;
    if idx < 0 {
        idx = 7;
    }
    table[(idx & 7) as usize]
}

/// `0x007824f0`: true when `active` holds and the y coordinate `y`
/// lies within `[top + 2 + off, bottom - 3 + off]` (all signed 16-bit).
pub fn w19a02_y_in_inset_band(active: bool, top: i16, bottom: i16, off: i16, y: i16) -> bool {
    if !active {
        return false;
    }
    let lo = i32::from(top) + 2 + i32::from(off);
    let hi = i32::from(bottom) - 3 + i32::from(off);
    lo <= i32::from(y) && i32::from(y) <= hi
}

/// `0x00749610`: chooses the small cue index (2..=6) from the sprite
/// kind `kind` and flag word `flags`; the two predicate flags map to 5/4.
pub fn w19a02_cue_index(is_a: bool, is_b: bool, kind: i16, flags: u32) -> u32 {
    if is_a {
        return 5;
    }
    if is_b {
        return 4;
    }
    if matches!(kind, 1 | 2 | 3 | 0xd) {
        return 3;
    }
    if matches!(kind, 9..=11) {
        if (flags as i32) < 0 {
            return 4;
        }
        if flags & 0x4000_0000 != 0 {
            return 6;
        }
        if flags & 0x2000_0000 != 0 {
            return 3;
        }
    }
    2
}

/// `0x00785620`: resolves a table value from two selector bytes
/// (`0xff` = none -> -1) through `table` and returns the matching
/// candidate id from `candidates` (`(valid, id)` pairs in list order), or
/// `0xffff` when none matches.  Value `0xe5` yields to the second choice.
pub fn w19a02_resolve_candidate(
    sel_a: u8,
    sel_b: u8,
    table: &[i16; 256],
    candidates: &[(bool, i16)],
) -> u16 {
    let mut a = table[sel_a as usize];
    let b = if sel_b == 0xff {
        -1
    } else {
        table[sel_b as usize]
    };
    if a == -1 || (b != -1 && a == 0xe5) {
        a = b;
    }
    if candidates.iter().any(|&(valid, id)| valid && id == a) {
        a as u16
    } else {
        0xffff
    }
}

#[cfg(test)]
mod tests_wave19_agent02 {
    use super::*;

    #[test]
    fn allowed() {
        assert!(w19a02_code_or_flag_allowed(1, 0));
        assert!(w19a02_code_or_flag_allowed(0, 0x80));
        assert!(w19a02_code_or_flag_allowed(0, 0x83));
        assert!(!w19a02_code_or_flag_allowed(0, 0x84));
        assert!(!w19a02_code_or_flag_allowed(0, 0x7f));
    }

    #[test]
    fn record() {
        let mut t = [[0u32; 4]; 8];
        for (i, r) in t.iter_mut().enumerate() {
            *r = [i as u32; 4];
        }
        let f = |c: u32| [c; 4];
        assert_eq!(w19a02_record_for_code(false, 5, |_| 3, &t, f), [5; 4]);
        assert_eq!(w19a02_record_for_code(true, 5, |_| 3, &t, f), [2; 4]);
        assert_eq!(w19a02_record_for_code(true, 5, |_| 0, &t, f), [7; 4]);
        assert_eq!(
            w19a02_record_for_code(true, 0x81, |c| if c == 13 { 8 } else { 1 }, &t, f),
            [7; 4]
        );
    }

    #[test]
    fn band() {
        assert!(!w19a02_y_in_inset_band(false, 0, 100, 0, 50));
        assert!(w19a02_y_in_inset_band(true, 0, 100, 10, 12));
        assert!(!w19a02_y_in_inset_band(true, 0, 100, 10, 11));
        assert!(w19a02_y_in_inset_band(true, 0, 100, 10, 107));
        assert!(!w19a02_y_in_inset_band(true, 0, 100, 10, 108));
    }

    #[test]
    fn cue() {
        assert_eq!(w19a02_cue_index(true, true, 0, 0), 5);
        assert_eq!(w19a02_cue_index(false, true, 0, 0), 4);
        assert_eq!(w19a02_cue_index(false, false, 0xd, 0), 3);
        assert_eq!(w19a02_cue_index(false, false, 9, 0x8000_0000), 4);
        assert_eq!(w19a02_cue_index(false, false, 10, 0x4000_0000), 6);
        assert_eq!(w19a02_cue_index(false, false, 11, 0x2000_0000), 3);
        assert_eq!(w19a02_cue_index(false, false, 9, 0), 2);
        assert_eq!(w19a02_cue_index(false, false, 5, 0), 2);
    }

    #[test]
    fn resolve() {
        let mut t = [-1i16; 256];
        t[1] = 0xe5;
        t[2] = 7;
        t[3] = 9;
        let c = [(true, 7), (false, 9)];
        assert_eq!(w19a02_resolve_candidate(1, 2, &t, &c), 7);
        assert_eq!(w19a02_resolve_candidate(0, 2, &t, &c), 7);
        assert_eq!(w19a02_resolve_candidate(3, 0xff, &t, &c), 0xffff);
        assert_eq!(w19a02_resolve_candidate(0, 0xff, &t, &c), 0xffff);
    }
}

// ===========================================================================
// Merged from src/wave19/agent03.rs
// ===========================================================================
// Wave 19 translation batch 03: dialog/listbox/scrollbar geometry helpers.
//
// Dialog records use the retail layout: `+4/+6/+8/+10` i16 bounds
// (left, top, right, bottom), `+0x30` flags, `+0x3c` control type.

/// `0x00957300`: maps a key/event code to a compact index; unknown codes map to 0.
pub fn w19a03_event_code_index(code: u32) -> u32 {
    match code {
        3 => 1,
        4 => 2,
        5 => 3,
        6 => 4,
        7 => 5,
        8 => 8,
        0xe => 9,
        0xf => 10,
        0x10 => 0xb,
        0x11 => 0xc,
        0x15 => 0xd,
        0x16 => 0xe,
        0x17 => 0xf,
        0x18 => 0x10,
        0x19 => 0x11,
        0x1b => 0x12,
        0x1c => 0x13,
        0x1d => 0x14,
        0x1e => 0x15,
        0x1f => 0x17,
        _ => 0,
    }
}

/// `0x009767d0`: writes `[l - m, t - m, r + m, b + m]` into `out`.
pub fn w19a03_inflate_rect(out: &mut [i32; 4], l: i32, t: i32, r: i32, b: i32, margin: i32) {
    out[0] = l.wrapping_sub(margin);
    out[1] = t.wrapping_sub(margin);
    out[2] = r.wrapping_add(margin);
    out[3] = b.wrapping_add(margin);
}

/// Listbox fields read by `w19a03_listbox_hit_index`.
#[derive(Clone, Copy, Debug, Default)]
#[allow(non_camel_case_types)]
pub struct w19a03_ListBox {
    pub left: i16,
    pub top: i16,
    pub right: i16,
    pub bottom: i16,
    pub flags: u32,
    pub ctrl_type: i16,
    pub count: u8,         // +0x60
    pub row_height: u8,    // +0x63
    pub visible_rows: u8,  // +0x64
    pub first_visible: u8, // +0x67
    pub selected: u8,      // +0x68
    pub type13_flags: u8,  // +0x66
}

/// `0x009792b0`: maps a mouse position (`mx`, `my`) to a row index.  Clears
/// flag `0x40000000` unless inside the interior x range, in which case it
/// sets it.  Result is clamped to `count - 1`.
pub fn w19a03_listbox_hit_index(lb: &mut w19a03_ListBox, mx: i16, my: i16) -> u8 {
    let header = if lb.ctrl_type == 0xd && lb.type13_flags & 1 == 0 {
        lb.row_height as i32 + lb.top as i32
    } else {
        lb.top as i32
    };
    let mut idx: u8;
    if (my as i32) < header {
        let (sel, first) = (lb.selected, lb.first_visible);
        lb.flags &= 0xbfff_ffff;
        idx = first;
        if sel <= first {
            idx = sel.saturating_sub(1);
        }
    } else if lb.bottom < my {
        idx = lb.selected;
        lb.flags &= 0xbfff_ffff;
        let last_vis = (lb.first_visible as i32 - 1) + lb.visible_rows as i32;
        if (idx as i32) < last_vis {
            idx = (lb.visible_rows.wrapping_sub(1)).wrapping_add(lb.first_visible);
        } else if (idx as i32) < lb.count as i32 - 1 {
            idx += 1;
        }
    } else {
        if lb.left < mx && mx < lb.right {
            lb.flags |= 0x4000_0000;
        }
        let rh = lb.row_height as i32;
        let q = if rh == 0 {
            0
        } else {
            ((my as i32 - header) as i16 as i32) / rh
        };
        idx = (q as u8).wrapping_add(lb.first_visible);
    }
    let last = lb.count.wrapping_sub(1);
    if idx < lb.count { idx } else { last }
}

/// `0x00982690`: scrollbar track length.  `g` supplies the retail globals
/// `[dd74, dd78, dd88, dd8c, ddcc, ddd0]` (caller-supplied widths).
/// Type 6 is horizontal, all other types vertical.
pub fn w19a03_scrollbar_length(flags: u32, ctrl_type: i16, bounds: [i16; 4], g: [i32; 6]) -> i16 {
    let [l, t, r, b] = bounds;
    let [g74, g78, g88, g8c, gcc, gd0] = g;
    let small = flags & 0x400_0000 != 0;
    let v = if ctrl_type != 6 {
        if small {
            (b as i32 - t as i32) - gd0 + 1
        } else {
            ((b as i32 + g74 * -2) - t as i32) - g8c - 5
        }
    } else if small {
        (r as i32 - l as i32) - gcc + 1
    } else {
        ((r as i32 + g78 * -2) - l as i32) - g88 - 1
    };
    v as i16
}

/// `0x00983010`: decrements the repeat counter at `+0x61`.  When it hits 0
/// it is reloaded with 3 (or 1 when `mask & 0x60000000 != 0`) and the caller
/// must run the step callback; the returned flag is true when the callback
/// must be followed by the non-returning stop (`mask & flags != 0`).
/// Returns `(fire, stop)`.
pub fn w19a03_repeat_tick(counter: &mut u8, mask: u32, flags: u32) -> (bool, bool) {
    *counter = counter.wrapping_sub(1);
    if *counter != 0 {
        return (false, false);
    }
    *counter = if mask & 0x6000_0000 == 0 { 3 } else { 1 };
    (true, mask & flags != 0)
}

/// `0x0097b260`: palette/state index passed to the draw-state setter.
/// `is_a`/`is_b` are the results of the two predicate callees.
pub fn w19a03_draw_state_index(is_a: bool, is_b: bool, ctrl_type: i16, flags: u32) -> u32 {
    if is_a {
        return 5;
    }
    if is_b {
        return 4;
    }
    if matches!(ctrl_type, 1 | 2 | 3 | 0xd) {
        return 3;
    }
    if matches!(ctrl_type, 9..=11) {
        if (flags as i32) < 0 {
            return 4;
        }
        if flags & 0x4000_0000 != 0 {
            return 6;
        }
        if flags & 0x2000_0000 != 0 {
            return 3;
        }
    }
    2
}

/// `0x0097b490`: when `pred` is true, sets the draw color and runs `draw`.
pub fn w19a03_draw_if(pred: bool, color: u8, color_slot: &mut u8, draw: impl FnOnce()) {
    if pred {
        *color_slot = color;
        draw();
    }
}

/// `0x0097b4c0`: two-pass frame: returns `(color, rect)` pairs, first the
/// rect inset by 1 in `color`, then the full rect in `outer_color`.
pub fn w19a03_frame_rects(bounds: [i16; 4], color: u8, outer_color: u8) -> [(u8, [i16; 4]); 2] {
    let [l, t, r, b] = bounds;
    [
        (color, [l + 1, t + 1, r - 1, b - 1]),
        (outer_color, [l, t, r, b]),
    ]
}

/// `0x00981530`: collects the dialog and its visible children overlapping
/// `area = [x0, y0, x1, y1]` (retail `param_2` ints `[0]=x0,[1]=y0,[2]=x1,[3]=y1`).
/// Children are `(bounds, visible, parent_offset)`; returns `None` when the
/// dialog itself does not overlap, else the picked child indices.
pub fn w19a03_collect_overlapping(
    dialog: [i16; 4],
    children: &[([i16; 4], bool, (i16, i16))],
    area: [i32; 4],
) -> Option<Vec<usize>> {
    if !(dialog[0] as i32 <= area[2]
        && dialog[1] as i32 <= area[3]
        && area[0] <= dialog[2] as i32
        && area[1] <= dialog[3] as i32)
    {
        return None;
    }
    let mut out = Vec::new();
    for (i, (b, visible, (ox, oy))) in children.iter().enumerate() {
        if !*visible {
            continue;
        }
        let (ox, oy) = (*ox as i32, *oy as i32);
        if b[0] as i32 + ox <= area[2]
            && b[1] as i32 + oy <= area[3]
            && area[0] <= b[2] as i32 + ox
            && area[1] <= b[3] as i32 + oy
        {
            out.push(i);
        }
    }
    Some(out)
}

/// `0x00981f80`: horizontal slider/bar draw ops `(glyph, x)` for a dialog
/// whose x-origin is `left`.  `g = [dd78, dd84]` are the cap and tile widths.
/// Returns an empty list when the bar is too narrow.  `flags & 0x4000000`
/// delegates to `0x009821e0` (returned as `None`).
pub fn w19a03_bar_draw_ops(
    flags: u32,
    left: u16,
    right: i16,
    dd78: u16,
    dd84: u16,
    is_a: bool,
) -> Option<Vec<(u8, i32)>> {
    if flags & 0x400_0000 != 0 {
        return None;
    }
    let cap = dd78 as i32;
    let tile = dd84 as i32;
    let l = left as i32;
    if (right as i32 - left as i16 as i32) < (tile + cap) * 2 {
        return Some(Vec::new());
    }
    let n = !is_a as u8;
    let mut ops = Vec::new();
    ops.push((
        if flags & 0x4000_0000 == 0 {
            0x16 + n
        } else {
            0x17 + n
        },
        l,
    ));
    let mut off = cap;
    ops.push((0x20, l + off));
    let mut cnt = (right as i32 - 2 * cap - left as i16 as i32) / tile - 2;
    while cnt != 0 {
        off += tile;
        ops.push((0x21, l + off));
        cnt -= 1;
    }
    ops.push((0x22, l + off + tile));
    let x = (cap + off + tile) as i16 as i32 + l;
    ops.push((
        if flags & 0x2000_0000 == 0 {
            0x19 + n
        } else {
            0x1a + n
        },
        x,
    ));
    Some(ops)
}

#[cfg(test)]
mod tests_wave19_agent03 {
    use super::*;

    #[test]
    fn event_map() {
        assert_eq!(w19a03_event_code_index(3), 1);
        assert_eq!(w19a03_event_code_index(8), 8);
        assert_eq!(w19a03_event_code_index(0x1f), 0x17);
        assert_eq!(w19a03_event_code_index(0x20), 0);
        assert_eq!(w19a03_event_code_index(9), 0);
    }

    #[test]
    fn inflate() {
        let mut r = [0; 4];
        w19a03_inflate_rect(&mut r, 10, 20, 30, 40, 3);
        assert_eq!(r, [7, 17, 33, 43]);
    }

    fn lb() -> w19a03_ListBox {
        w19a03_ListBox {
            left: 0,
            top: 10,
            right: 100,
            bottom: 90,
            flags: 0,
            ctrl_type: 1,
            count: 20,
            row_height: 10,
            visible_rows: 8,
            first_visible: 2,
            selected: 5,
            type13_flags: 0,
        }
    }

    #[test]
    fn hit_index() {
        let mut l = lb();
        assert_eq!(w19a03_listbox_hit_index(&mut l, 50, 35), 4);
        assert_ne!(l.flags & 0x4000_0000, 0);
        let mut l = lb();
        assert_eq!(w19a03_listbox_hit_index(&mut l, 50, 0), 2); // above: first_visible
        let mut l = lb();
        l.selected = 1;
        assert_eq!(w19a03_listbox_hit_index(&mut l, 50, 0), 0);
        let mut l = lb();
        assert_eq!(w19a03_listbox_hit_index(&mut l, 50, 200), 9); // below: last visible
        let mut l = lb();
        l.selected = 12;
        assert_eq!(w19a03_listbox_hit_index(&mut l, 50, 200), 13);
        let mut l = lb();
        assert_eq!(w19a03_listbox_hit_index(&mut l, 50, 89), 9);
        let mut l = lb();
        l.count = 3;
        assert_eq!(w19a03_listbox_hit_index(&mut l, 50, 85), 2);
    }

    #[test]
    fn scroll_len() {
        let b = [0, 0, 200, 100];
        let g = [2, 3, 4, 5, 6, 7];
        assert_eq!(w19a03_scrollbar_length(0x400_0000, 1, b, g), 94);
        assert_eq!(w19a03_scrollbar_length(0, 1, b, g), 100 - 4 - 5 - 5);
        assert_eq!(w19a03_scrollbar_length(0x400_0000, 6, b, g), 195);
        assert_eq!(w19a03_scrollbar_length(0, 6, b, g), 200 - 6 - 4 - 1);
    }

    #[test]
    fn repeat() {
        let mut c = 2;
        assert_eq!(w19a03_repeat_tick(&mut c, 0, 0), (false, false));
        assert_eq!(w19a03_repeat_tick(&mut c, 0, 1), (true, false));
        assert_eq!(c, 3);
        let mut c = 1;
        assert_eq!(
            w19a03_repeat_tick(&mut c, 0x2000_0000, 0x2000_0000),
            (true, true)
        );
        assert_eq!(c, 1);
    }

    #[test]
    fn state_index() {
        assert_eq!(w19a03_draw_state_index(true, true, 0, 0), 5);
        assert_eq!(w19a03_draw_state_index(false, true, 0, 0), 4);
        assert_eq!(w19a03_draw_state_index(false, false, 0xd, 0), 3);
        assert_eq!(w19a03_draw_state_index(false, false, 9, 0x8000_0000), 4);
        assert_eq!(w19a03_draw_state_index(false, false, 10, 0x4000_0000), 6);
        assert_eq!(w19a03_draw_state_index(false, false, 11, 0x2000_0000), 3);
        assert_eq!(w19a03_draw_state_index(false, false, 11, 0), 2);
        assert_eq!(w19a03_draw_state_index(false, false, 5, 0), 2);
    }

    #[test]
    fn draw_if_and_frame() {
        let mut slot = 0u8;
        let mut hit = false;
        w19a03_draw_if(false, 9, &mut slot, || hit = true);
        assert!(!hit && slot == 0);
        w19a03_draw_if(true, 9, &mut slot, || hit = true);
        assert!(hit && slot == 9);
        let f = w19a03_frame_rects([0, 0, 10, 20], 1, 2);
        assert_eq!(f[0], (1, [1, 1, 9, 19]));
        assert_eq!(f[1], (2, [0, 0, 10, 20]));
    }

    #[test]
    fn overlap() {
        let kids = [
            ([0, 0, 10, 10], true, (100, 100)),
            ([0, 0, 10, 10], false, (0, 0)),
            ([0, 0, 10, 10], true, (0, 0)),
        ];
        assert_eq!(
            w19a03_collect_overlapping([0, 0, 50, 50], &kids, [5, 5, 20, 20]),
            Some(vec![2])
        );
        assert_eq!(
            w19a03_collect_overlapping([0, 0, 50, 50], &kids, [60, 0, 70, 5]),
            None
        );
    }

    #[test]
    fn bar_ops() {
        assert!(w19a03_bar_draw_ops(0x400_0000, 0, 100, 4, 8, true).is_none());
        assert_eq!(w19a03_bar_draw_ops(0, 0, 10, 4, 8, true), Some(vec![]));
        let ops = w19a03_bar_draw_ops(0, 0, 40, 4, 8, false).unwrap();
        assert_eq!(ops[0], (0x17, 0));
        assert_eq!(ops[1], (0x20, 4));
        assert_eq!(ops[2], (0x21, 12));
        assert_eq!(ops[3], (0x21, 20));
        assert_eq!(ops[4], (0x22, 28));
        assert_eq!(ops[5], (0x1a, 32));
    }
}

// ===========================================================================
// Merged from src/wave19/agent04.rs
// ===========================================================================
// Wave 19 translation batch 04: block variance kernels (w x h luma blocks).

/// Shared body of the variance kernels: returns `(variance, sse)` where
/// `variance = sse - (sum * sum) / (w * h)` (the retail 64-bit signed divide
/// helper is a plain truncating division).
pub fn w19a04_variance(
    src: &[u8],
    src_stride: usize,
    refp: &[u8],
    ref_stride: usize,
    w: usize,
    h: usize,
) -> (i32, i32) {
    let mut sum: i32 = 0;
    let mut sse: i32 = 0;
    for y in 0..h {
        for x in 0..w {
            let d = src[y * src_stride + x] as i32 - refp[y * ref_stride + x] as i32;
            sum = sum.wrapping_add(d);
            sse = sse.wrapping_add(d.wrapping_mul(d));
        }
    }
    let n = (w * h) as i64;
    let sq = (sum as i64) * (sum as i64);
    ((sse as i64 - sq / n) as i32, sse)
}

/// `0x00a35d50`: 16x16 block variance; returns `(variance, sse)` (divisor 256).
pub fn w19a04_variance_16x16(
    src: &[u8],
    src_stride: usize,
    refp: &[u8],
    ref_stride: usize,
) -> (i32, i32) {
    w19a04_variance(src, src_stride, refp, ref_stride, 16, 16)
}

/// `0x00a35e00`: 16x32 block variance; returns `(variance, sse)` (divisor 512).
pub fn w19a04_variance_16x32(
    src: &[u8],
    src_stride: usize,
    refp: &[u8],
    ref_stride: usize,
) -> (i32, i32) {
    w19a04_variance(src, src_stride, refp, ref_stride, 16, 32)
}

/// `0x00a35eb0`: 16x8 block variance; returns `(variance, sse)` (divisor 128).
pub fn w19a04_variance_16x8(
    src: &[u8],
    src_stride: usize,
    refp: &[u8],
    ref_stride: usize,
) -> (i32, i32) {
    w19a04_variance(src, src_stride, refp, ref_stride, 16, 8)
}

/// `0x00a35f60`: 32x16 block variance; returns `(variance, sse)` (divisor 512).
pub fn w19a04_variance_32x16(
    src: &[u8],
    src_stride: usize,
    refp: &[u8],
    ref_stride: usize,
) -> (i32, i32) {
    w19a04_variance(src, src_stride, refp, ref_stride, 32, 16)
}

/// `0x00a36010`: 32x32 block variance; returns `(variance, sse)` (divisor 1024).
pub fn w19a04_variance_32x32(
    src: &[u8],
    src_stride: usize,
    refp: &[u8],
    ref_stride: usize,
) -> (i32, i32) {
    w19a04_variance(src, src_stride, refp, ref_stride, 32, 32)
}

/// `0x00a360c0`: 32x64 block variance; returns `(variance, sse)` (divisor 2048).
pub fn w19a04_variance_32x64(
    src: &[u8],
    src_stride: usize,
    refp: &[u8],
    ref_stride: usize,
) -> (i32, i32) {
    w19a04_variance(src, src_stride, refp, ref_stride, 32, 64)
}

/// `0x00a362b0`: 64x32 block variance; returns `(variance, sse)` (divisor 2048).
pub fn w19a04_variance_64x32(
    src: &[u8],
    src_stride: usize,
    refp: &[u8],
    ref_stride: usize,
) -> (i32, i32) {
    w19a04_variance(src, src_stride, refp, ref_stride, 64, 32)
}

/// `0x00a36360`: 64x64 block variance; returns `(variance, sse)` (divisor 4096).
pub fn w19a04_variance_64x64(
    src: &[u8],
    src_stride: usize,
    refp: &[u8],
    ref_stride: usize,
) -> (i32, i32) {
    w19a04_variance(src, src_stride, refp, ref_stride, 64, 64)
}

/// `0x00a36170`: 4x4 block variance; returns `(variance, sse)` (divisor 16).
pub fn w19a04_variance_4x4(
    src: &[u8],
    src_stride: usize,
    refp: &[u8],
    ref_stride: usize,
) -> (i32, i32) {
    w19a04_variance(src, src_stride, refp, ref_stride, 4, 4)
}

/// `0x00a36210`: 4x8 block variance; returns `(variance, sse)` (divisor 32).
pub fn w19a04_variance_4x8(
    src: &[u8],
    src_stride: usize,
    refp: &[u8],
    ref_stride: usize,
) -> (i32, i32) {
    w19a04_variance(src, src_stride, refp, ref_stride, 4, 8)
}

/// `0x00a36520`: 8x4 block variance; returns `(variance, sse)` (divisor 32).
pub fn w19a04_variance_8x4(
    src: &[u8],
    src_stride: usize,
    refp: &[u8],
    ref_stride: usize,
) -> (i32, i32) {
    w19a04_variance(src, src_stride, refp, ref_stride, 8, 4)
}

/// `0x00a36620`: 8x8 block variance; returns `(variance, sse)` (divisor 64).
pub fn w19a04_variance_8x8(
    src: &[u8],
    src_stride: usize,
    refp: &[u8],
    ref_stride: usize,
) -> (i32, i32) {
    w19a04_variance(src, src_stride, refp, ref_stride, 8, 8)
}

/// `0x00a8fab0`: byte RGB to float RGBA in `[0,1]`, alpha fixed at 1.0.
pub fn w19a04_rgb_to_float(out: &mut [f32; 4], r: u8, g: u8, b: u8) {
    out[3] = 1.0;
    out[0] = r as f32 / 255.0;
    out[1] = g as f32 / 255.0;
    out[2] = b as f32 / 255.0;
}

#[cfg(test)]
mod tests_wave19_agent04 {
    use super::*;

    fn exp(a: &[u8], b: &[u8], n: usize) -> (i32, i32) {
        let mut sum = 0i64;
        let mut sse = 0i64;
        for i in 0..n {
            let d = a[i] as i64 - b[i] as i64;
            sum += d;
            sse += d * d;
        }
        ((sse - sum * sum / n as i64) as i32, sse as i32)
    }

    #[test]
    fn variance_kernels() {
        let a: Vec<u8> = (0..4096).map(|i| (i * 7 % 251) as u8).collect();
        let b: Vec<u8> = (0..4096).map(|i| (i * 13 % 241) as u8).collect();
        let (v, s) = w19a04_variance_16x16(&a, 16, &b, 16);
        assert_eq!((v, s), exp(&a, &b, 256));
        let (v, s) = w19a04_variance_16x32(&a, 16, &b, 16);
        assert_eq!((v, s), exp(&a, &b, 512));
        let (v, s) = w19a04_variance_16x8(&a, 16, &b, 16);
        assert_eq!((v, s), exp(&a, &b, 128));
        let (v, s) = w19a04_variance_32x16(&a, 32, &b, 32);
        assert_eq!((v, s), exp(&a, &b, 512));
        let (v, s) = w19a04_variance_32x32(&a, 32, &b, 32);
        assert_eq!((v, s), exp(&a, &b, 1024));
        let (v, s) = w19a04_variance_32x64(&a, 32, &b, 32);
        assert_eq!((v, s), exp(&a, &b, 2048));
        let (v, s) = w19a04_variance_64x32(&a, 64, &b, 64);
        assert_eq!((v, s), exp(&a, &b, 2048));
        let (v, s) = w19a04_variance_64x64(&a, 64, &b, 64);
        assert_eq!((v, s), exp(&a, &b, 4096));
        let (v, s) = w19a04_variance_4x4(&a, 4, &b, 4);
        assert_eq!((v, s), exp(&a, &b, 16));
        let (v, s) = w19a04_variance_4x8(&a, 4, &b, 4);
        assert_eq!((v, s), exp(&a, &b, 32));
        let (v, s) = w19a04_variance_8x4(&a, 8, &b, 8);
        assert_eq!((v, s), exp(&a, &b, 32));
        let (v, s) = w19a04_variance_8x8(&a, 8, &b, 8);
        assert_eq!((v, s), exp(&a, &b, 64));
    }

    #[test]
    fn variance_constant_diff_is_zero() {
        let a = [10u8; 16];
        let b = [3u8; 16];
        assert_eq!(w19a04_variance_4x4(&a, 4, &b, 4), (0, 16 * 49));
    }

    #[test]
    fn strided_rows() {
        let a = [1u8, 2, 0, 0, 3, 4, 0, 0, 5, 6, 0, 0, 7, 8, 0, 0];
        let b = [0u8; 16];
        let (v, s) = w19a04_variance_4x4(&a, 4, &b, 4);
        assert_eq!(s, 1 + 4 + 9 + 16 + 25 + 36 + 49 + 64);
        assert_eq!(v, s - 36 * 36 / 16);
    }

    #[test]
    fn rgb() {
        let mut o = [0.0f32; 4];
        w19a04_rgb_to_float(&mut o, 255, 0, 51);
        assert_eq!(o, [1.0, 0.0, 0.2, 1.0]);
    }
}

// ===========================================================================
// Merged from src/wave19/agent05.rs
// ===========================================================================
// Wave 19 translation batch 05: CRT time/stat helpers and wait-timeout
// conversions (merge target `path.rs`).

/// Cumulative day table for a non-leap year (CRT `_days`, base -1).
pub const W19A05_DAYS: [i32; 13] = [-1, 30, 58, 89, 119, 150, 180, 211, 242, 272, 303, 333, 364];
/// Cumulative day table for a leap year (CRT `_lpdays`, base -1).
pub const W19A05_LPDAYS: [i32; 13] = [-1, 30, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334, 365];

/// Seconds between 1601-01-01 and 1970-01-01 in 100 ns units.
const W19A05_FILETIME_EPOCH: i64 = 0x019d_b1de_d53e_8000;

fn w19a05_leap_days(year: i32) -> i32 {
    // Days from 1900 baseline used by the year split (CRT helper).
    let y1 = year.wrapping_sub(1);
    (y1.wrapping_add((y1 >> 31) & 3) >> 2) - 17 + ((year.wrapping_add(299)) / 400 - y1 / 100)
}

fn w19a05_is_leap(year: i32) -> bool {
    if year % 4 == 0 && year % 100 != 0 {
        return true;
    }
    year.wrapping_add(0x76c) % 400 == 0
}

/// `0x00b2b5b0`: seconds-to-minutes conversion.  When the 64-bit input is
/// zero the current time (`now`, a 64-bit tick count) is substituted; the
/// result is `t / 60` (signed 64-bit division).
pub fn w19a05_ticks_to_minutes(t: i64, now: impl FnOnce() -> i64) -> i64 {
    let t = if t == 0 { now() } else { t };
    t / 0x3c
}

/// Microsecond conversion shared by the wait helpers: `t / 1_000_000`.
fn w19a05_to_micro_units(t: i64) -> i64 {
    t / 1_000_000
}

/// `0x00c54580`: waits for a stream to reach the stored position.  Returns
/// `true` when the stream is already at or past `target` (`+0x10`, 64-bit);
/// otherwise seeks (`seek(target_lo, target_hi, handle)`) and returns
/// whether `wait(handle, timeout / 1e6)` returned 0.  `pos` is vtable
/// slot `+0x20`.
pub fn w19a05_wait_for_position(
    target: u64,
    handle: u32,
    timeout: i64,
    pos: impl FnOnce() -> u64,
    seek: impl FnOnce(u32, u32, u32),
    wait: impl FnOnce(u32, u32) -> i32,
) -> bool {
    if target <= pos() {
        return true;
    }
    seek(target as u32, (target >> 32) as u32, handle);
    wait(handle, w19a05_to_micro_units(timeout) as u32) == 0
}

/// Clamp used by the three wait wrappers: low 32 bits of the quotient,
/// or `0xffffffff` when the quotient does not fit zero high bits.
fn w19a05_micro_clamped(t: i64) -> u32 {
    let q = w19a05_to_micro_units(t);
    if (q >> 32) as i32 != 0 {
        0xffff_ffff
    } else {
        q as u32
    }
}

/// `0x00c665b0`: converts a 64-bit duration to microsecond units (clamped
/// to `u32::MAX`) and passes it to the OS sleep call.
pub fn w19a05_sleep_micro(t: i64, sleep: impl FnOnce(u32)) {
    sleep(w19a05_micro_clamped(t));
}

/// `0x00c66660`: same clamp as `w19a05_sleep_micro`; returns whether
/// `wait(handle, units)` returned 0.
pub fn w19a05_wait_handle_micro(handle: u32, t: i64, wait: impl FnOnce(u32, u32) -> i32) -> bool {
    wait(handle, w19a05_micro_clamped(t)) == 0
}

/// `0x00c686e0`: same clamp; returns whether `call(a, b, units, 0)` is
/// non-zero.
pub fn w19a05_wait_two_micro(
    a: u32,
    b: u32,
    t: i64,
    call: impl FnOnce(u32, u32, u32, u32) -> i32,
) -> bool {
    call(a, b, w19a05_micro_clamped(t), 0) != 0
}

/// `0x00c9d4b8`: `timespec_get`.  `base` must be 1 (TIME_UTC) and
/// `filetime` is the current FILETIME (100 ns since 1601).  Returns
/// `(tv_sec, tv_nsec)` or `None` (base != 1 or time beyond 32-bit range).
pub fn w19a05_timespec_get(base: i32, filetime: u64) -> Option<(i32, i32)> {
    if base != 1 {
        return None;
    }
    let t = (filetime as i64).wrapping_sub(W19A05_FILETIME_EPOCH);
    let nsec = (t % 10_000_000) as i32;
    if t < 0x004c_4b24_e140_4000 {
        Some(((t / 10_000_000) as i32, nsec.wrapping_mul(100)))
    } else {
        None
    }
}

/// `0x00c9c7c8`: splits a 64-bit seconds value into whole years since 1900
/// (returned) leaving the seconds-within-year in `*t`.  Sets `*leap` to 1
/// when the resulting year is a leap year.
pub fn w19a05_split_year(t: &mut i64, leap: &mut u8) -> i32 {
    let years = (*t / 0x01e1_3380) as i32;
    let mut year = years.wrapping_add(0x46);
    let days = w19a05_leap_days(year) as i64;
    *t = *t - years as i64 * 0x01e1_3380 - days * 0x15180;
    if *t < 0 {
        *t += 0x01e1_3380;
        year = years.wrapping_add(0x45);
        if w19a05_is_leap(year) {
            *t += 0x15180;
            *leap = 1;
        }
    } else if w19a05_is_leap(year) {
        *leap = 1;
    }
    year
}

/// `0x00c9c67c`: CRT `_gmtime64_s`.  Returns the `tm` as
/// `[sec, min, hour, mday, mon, year, wday, yday, isdst]` plus the status
/// (0 or EINVAL = 0x16).  On error every field is -1.
pub fn w19a05_gmtime(time: i64) -> ([i32; 9], u32) {
    let mut tm = [-1i32; 9];
    if !(-43200..0x7_9358_e1d0).contains(&time) {
        return (tm, 0x16);
    }
    let mut t = time;
    let mut leap = 0u8;
    tm[5] = w19a05_split_year(&mut t, &mut leap);
    let yday = (t / 0x15180) as i32;
    tm[7] = yday;
    let mut rem = t - yday as i64 * 0x15180;
    let table = if leap == 0 {
        &W19A05_DAYS
    } else {
        &W19A05_LPDAYS
    };
    let mut m = 1;
    while table[m] < yday {
        m += 1;
    }
    tm[4] = m as i32 - 1;
    tm[3] = yday - table[m - 1];
    tm[6] = ((time / 0x15180) as i32 + 4) % 7;
    let hour = (rem / 0xe10) as i32;
    tm[2] = hour;
    rem -= hour as i64 * 0xe10;
    let min = (rem / 0x3c) as i32;
    tm[1] = min;
    tm[0] = rem as i32 - min * 0x3c;
    tm[8] = 0;
    (tm, 0)
}

/// `0x00b905c0`: Win32 error to errno map used by `_stat` when the file
/// attribute query fails (access/sharing 5,0x20,0x21,0x24 -> EACCES 13;
/// 8 -> ENOMEM 12; 0x6f -> ERANGE 0x26; anything else ENOENT 2).
pub fn w19a05_stat_errno(last_error: u32) -> u32 {
    match last_error {
        5 | 0x20 | 0x21 | 0x24 => 13,
        8 => 12,
        0x6f => 0x26,
        _ => 2,
    }
}

/// `0x00b905c0`: fills the 0x30-byte stat record (as 12 words) from
/// `WIN32_FILE_ATTRIBUTE_DATA`: attributes, creation/access/write FILETIMEs
/// and the low size.  Word 1 = 0x1000000, word 2 = 1 (nlink), the mode is
/// the u16 at byte 6 (dir 0x4100 else 0x8100, `|0x80` when not read-only);
/// words 6..8, 8..10, 10..12 are atime, mtime, ctime in Unix seconds.
pub fn w19a05_stat_fill(
    attrs: u32,
    creation: u64,
    access: u64,
    write: u64,
    size_low: u32,
) -> [u32; 12] {
    let mut s = [0u32; 12];
    let mut mode: u16 = if attrs & 0x10 == 0 { 0x8100 } else { 0x4100 };
    if attrs & 1 == 0 {
        mode |= 0x80;
    }
    s[1] = 0x100_0000;
    s[1] = (s[1] & 0x0000_ffff) | ((mode as u32) << 16);
    s[2] = 1;
    s[5] = size_low;
    let conv = |ft: u64| ((ft as i64).wrapping_sub(W19A05_FILETIME_EPOCH) / 10_000_000) as u64;
    for (i, ft) in [access, write, creation].into_iter().enumerate() {
        let v = conv(ft);
        s[6 + 2 * i] = v as u32;
        s[7 + 2 * i] = (v >> 32) as u32;
    }
    s
}

#[cfg(test)]
mod tests_wave19_agent05 {
    use super::*;

    #[test]
    fn minutes_and_now() {
        assert_eq!(w19a05_ticks_to_minutes(120, || 0), 2);
        assert_eq!(w19a05_ticks_to_minutes(0, || 600), 10);
    }

    #[test]
    fn wait_wrappers() {
        let mut got = 0;
        w19a05_sleep_micro(5_000_000, |u| got = u);
        assert_eq!(got, 5);
        w19a05_sleep_micro(i64::MAX, |u| got = u);
        assert_eq!(got, u32::MAX);
        assert!(w19a05_wait_handle_micro(1, 3_000_000, |h, u| (h + u)
            as i32
            - 4));
        assert!(w19a05_wait_two_micro(
            1,
            2,
            2_000_000,
            |a, b, u, z| (a + b + u + z) as i32
        ));
    }

    #[test]
    fn wait_position() {
        assert!(w19a05_wait_for_position(
            5,
            9,
            0,
            || 7,
            |_, _, _| panic!(),
            |_, _| 1
        ));
        let mut seeked = None;
        let r = w19a05_wait_for_position(
            0x1_0000_0002,
            9,
            4_000_000,
            || 0,
            |l, h, hd| seeked = Some((l, h, hd)),
            |h, u| {
                assert_eq!((h, u), (9, 4));
                0
            },
        );
        assert!(r);
        assert_eq!(seeked, Some((2, 1, 9)));
    }

    #[test]
    fn timespec() {
        assert_eq!(w19a05_timespec_get(2, 0), None);
        let ft = (W19A05_FILETIME_EPOCH + 3 * 10_000_000 + 5) as u64;
        assert_eq!(w19a05_timespec_get(1, ft), Some((3, 500)));
        assert_eq!(w19a05_timespec_get(1, i64::MAX as u64), None);
    }

    #[test]
    fn gmtime_epoch_and_leap() {
        let (tm, rc) = w19a05_gmtime(0);
        assert_eq!(rc, 0);
        assert_eq!(tm, [0, 0, 0, 1, 0, 70, 4, 0, 0]);
        // 2000-03-01 12:34:56 UTC = 951914096
        let (tm, rc) = w19a05_gmtime(951_914_096);
        assert_eq!(rc, 0);
        assert_eq!(tm, [56, 34, 12, 1, 2, 100, 3, 60, 0]);
        let (tm, rc) = w19a05_gmtime(-1);
        assert_eq!((rc, tm[5], tm[3]), (0, 69, 31));
        let (tm, rc) = w19a05_gmtime(-50000);
        assert_eq!((rc, tm), (0x16, [-1; 9]));
        assert_eq!(w19a05_gmtime(0x7_9358_e1d0).1, 0x16);
    }

    #[test]
    fn split_year_flags() {
        let mut t = 951_914_096;
        let mut leap = 0;
        assert_eq!(w19a05_split_year(&mut t, &mut leap), 100);
        assert_eq!(leap, 1);
        assert_eq!(t, 951_914_096 - 946_684_800);
    }

    #[test]
    fn stat_helpers() {
        assert_eq!(w19a05_stat_errno(5), 13);
        assert_eq!(w19a05_stat_errno(8), 12);
        assert_eq!(w19a05_stat_errno(0x6f), 0x26);
        assert_eq!(w19a05_stat_errno(2), 2);
        let ft = (W19A05_FILETIME_EPOCH + 10 * 10_000_000) as u64;
        let s = w19a05_stat_fill(0x10, ft, ft, ft, 77);
        assert_eq!(s[1] >> 16, 0x4180);
        assert_eq!((s[2], s[5], s[6], s[8], s[10]), (1, 77, 10, 10, 10));
        let s = w19a05_stat_fill(1, ft, ft, ft, 0);
        assert_eq!(s[1] >> 16, 0x8100);
    }
}

// ===========================================================================
// Merged from src/wave20/agent02.rs
// ===========================================================================
// Wave 20 translation batch 02: dialog control flag helpers and small UI math.

/// `0x009645e0`: control flag byte at `+0x34`, bit 0.
pub fn w20a02_ctrl_byte34_bit0(flags34: u32) -> u32 {
    flags34 & 1
}

/// `0x009645f0`: control flags at `+0x30`, bit 10.
pub fn w20a02_ctrl_flag30_bit10(flags30: u32) -> u32 {
    (flags30 >> 10) & 1
}

/// `0x00964630`: control flags at `+0x34`, bit 2.
pub fn w20a02_ctrl_flag34_bit2(flags34: u32) -> u32 {
    (flags34 >> 2) & 1
}

/// `0x00964650`: control flags at `+0x30`, bit 13.
pub fn w20a02_ctrl_flag30_bit13(flags30: u32) -> u32 {
    (flags30 >> 13) & 1
}

/// `0x00964c90`: recompute bit 0x400 of the control flags at `+0x30`.
/// Set when `flags34` bit 0 is clear, `flags30` bit 1 is set, and either the
/// control is its parent's focus (`is_parent_focus`) or it is type 1 with no
/// parent focus and the parent lacks bit 29.
pub fn w20a02_update_focus_flag(
    flags30: u32,
    flags34: u32,
    is_parent_focus: bool,
    ctrl_type: u16,
    parent_focus_null: bool,
    parent_flags30: u32,
) -> u32 {
    let set = (flags34 & 1) == 0
        && (flags30 >> 1) & 1 != 0
        && (is_parent_focus
            || (ctrl_type == 1 && parent_focus_null && parent_flags30 & 536_870_912u32 == 0));
    if set {
        flags30 | 0x400
    } else {
        flags30 & !0x400u32
    }
}

/// `0x0097a2e0`: caches a computed byte at `+0x68`; returns `Some(new)` when
/// it changed (caller then invokes the change callback), else `None`.
pub fn w20a02_update_cached_state(cached: &mut u8, computed: u8) -> Option<u8> {
    if *cached != computed {
        *cached = computed;
        Some(computed)
    } else {
        None
    }
}

/// `0x00976310`: selects one of four string/graphic ids by the first matching
/// predicate (checked in order 0x96_4680, 0x96_4610, 0x96_4540, else the last).
pub fn w20a02_select_by_state(preds: [bool; 3], ids: [u32; 4]) -> u32 {
    if preds[0] {
        ids[0]
    } else if preds[1] {
        ids[1]
    } else if preds[2] {
        ids[2]
    } else {
        ids[3]
    }
}

/// `0x006e47a0` (tail): final magnitude. `req` is the integer parameter/100,
/// `base` the callee value, `floor` the byte at `+0x5e`/100.  Returns `Some`
/// when above 0.1 (value is then divided by 5 when `slow`).
pub fn w20a02_effect_magnitude(req: i32, base: f32, floor_byte: u8, slow: bool) -> Option<f32> {
    let p = req as f32 / 100.0;
    let floor = f32::from(floor_byte) / 100.0;
    let m = base.max(floor);
    let mut v = if p <= m { m } else { p };
    if slow {
        v /= 5.0;
    }
    if 0.1 < v { Some(v) } else { None }
}

/// `0x006f74e0` (arithmetic): returns `(full_width, fill_width)` of a bar.
/// `left`/`right` are the i16 rect edges, `cur`/`max_unit` the first quotient
/// operands (`a = cur / max_unit`), `total` the divisor `local_28`.
pub fn w20a02_bar_widths(left: i16, right: i16, cur: i32, max_unit: i32, total: u32) -> (u32, u32) {
    let w = (i32::from(right) - i32::from(left) + 1) as u32;
    let a = cur.checked_div(max_unit).unwrap_or(0);
    let mut fill = match w.wrapping_mul(a as u32).checked_div(total) {
        Some(v) if v > w => return (w, w),
        Some(v) => v,
        None => 0,
    };
    if a != 0 && fill == 0 {
        fill = 1;
    }
    (w, fill)
}

#[cfg(test)]
mod tests_wave20_agent02 {
    use super::*;

    #[test]
    fn flag_getters() {
        assert_eq!(w20a02_ctrl_byte34_bit0(3), 1);
        assert_eq!(w20a02_ctrl_flag30_bit10(0x400), 1);
        assert_eq!(w20a02_ctrl_flag30_bit10(0x800), 0);
        assert_eq!(w20a02_ctrl_flag34_bit2(4), 1);
        assert_eq!(w20a02_ctrl_flag30_bit13(0x2000), 1);
    }

    #[test]
    fn focus_flag() {
        assert_eq!(w20a02_update_focus_flag(2, 0, true, 0, false, 0), 0x402);
        assert_eq!(w20a02_update_focus_flag(0x402, 1, true, 0, false, 0), 2);
        assert_eq!(w20a02_update_focus_flag(2, 0, false, 1, true, 0), 0x402);
        assert_eq!(
            w20a02_update_focus_flag(2, 0, false, 1, true, 536_870_912u32),
            2
        );
        assert_eq!(w20a02_update_focus_flag(0x400, 0, false, 0, false, 0), 0);
    }

    #[test]
    fn cached_and_select() {
        let mut c = 1u8;
        assert_eq!(w20a02_update_cached_state(&mut c, 1), None);
        assert_eq!(w20a02_update_cached_state(&mut c, 2), Some(2));
        assert_eq!(c, 2);
        let ids = [10, 20, 30, 40];
        assert_eq!(w20a02_select_by_state([false, true, true], ids), 20);
        assert_eq!(w20a02_select_by_state([false; 3], ids), 40);
        assert_eq!(w20a02_select_by_state([true; 3], ids), 10);
    }

    #[test]
    fn magnitude() {
        assert_eq!(w20a02_effect_magnitude(50, 0.2, 10, false), Some(0.5));
        assert_eq!(w20a02_effect_magnitude(0, 0.3, 50, false), Some(0.5));
        assert_eq!(w20a02_effect_magnitude(100, 0.2, 10, true), Some(0.2));
        assert_eq!(w20a02_effect_magnitude(5, 0.05, 0, false), None);
    }

    #[test]
    fn bar() {
        assert_eq!(w20a02_bar_widths(0, 9, 5, 1, 10), (10, 5));
        assert_eq!(w20a02_bar_widths(0, 9, 1, 1, 100), (10, 1));
        assert_eq!(w20a02_bar_widths(0, 9, 20, 1, 10), (10, 10));
        assert_eq!(w20a02_bar_widths(0, 9, 3, 1, 0), (10, 1));
        assert_eq!(w20a02_bar_widths(0, 9, 0, 1, 0), (10, 0));
    }
}

// ===========================================================================
// Merged from src/wave20/agent03.rs
// ===========================================================================
// Wave 20 translation batch 03: bilinear sub-pel block predictors and misc.
//
// The nine predictors share one two-pass kernel; the weight table at
// rdata VA 00d90260 is eight `(128-16i, 16i)` pairs (read from the image).

/// Bilinear weight pairs (patch 1.23.10 rdata table, 8 entries).
pub const W20A03_BILERP_WEIGHTS: [(u32, u32); 8] = [
    (128, 0),
    (112, 16),
    (96, 32),
    (80, 48),
    (64, 64),
    (48, 80),
    (32, 96),
    (16, 112),
];

/// Shared kernel of the predictors: horizontal pass over `h + 1` rows of
/// `w + 1` source pixels (`(a*w0 + b*w1 + 0x40) >> 7` into u16), then a
/// vertical pass blending adjacent rows with the second weight pair.
/// Panics if `fx`/`fy` >= 8 or `src` is too short.
pub fn w20a03_bilerp(
    src: &[u8],
    stride: usize,
    fx: usize,
    fy: usize,
    w: usize,
    h: usize,
) -> Vec<u8> {
    let (h0, h1) = W20A03_BILERP_WEIGHTS[fx];
    let mut tmp = vec![0u32; w * (h + 1)];
    for r in 0..=h {
        for c in 0..w {
            let a = src[r * stride + c] as u32;
            let b = src[r * stride + c + 1] as u32;
            tmp[r * w + c] = (a * h0 + b * h1 + 0x40) >> 7;
        }
    }
    let (v0, v1) = W20A03_BILERP_WEIGHTS[fy];
    let mut out = vec![0u8; w * h];
    for r in 0..h {
        for c in 0..w {
            out[r * w + c] = ((tmp[r * w + c] * v0 + tmp[(r + 1) * w + c] * v1 + 0x40) >> 7) as u8;
        }
    }
    out
}

/// `0x00a34130`: bilinear predictor for a 16x16 block; the result is handed to
/// `sink(block, stride = 16)` (the retail tail call, callee unresolved).
pub fn w20a03_bilerp_16x16(
    src: &[u8],
    stride: usize,
    fx: usize,
    fy: usize,
    mut sink: impl FnMut(&[u8], usize),
) {
    let out = w20a03_bilerp(src, stride, fx, fy, 16, 16);
    sink(&out, 16);
}

/// `0x00a34350`: bilinear predictor for a 16x32 block; the result is handed to
/// `sink(block, stride = 16)` (the retail tail call, callee unresolved).
pub fn w20a03_bilerp_16x32(
    src: &[u8],
    stride: usize,
    fx: usize,
    fy: usize,
    mut sink: impl FnMut(&[u8], usize),
) {
    let out = w20a03_bilerp(src, stride, fx, fy, 16, 32);
    sink(&out, 16);
}

/// `0x00a34570`: bilinear predictor for a 16x8 block; the result is handed to
/// `sink(block, stride = 16)` (the retail tail call, callee unresolved).
pub fn w20a03_bilerp_16x8(
    src: &[u8],
    stride: usize,
    fx: usize,
    fy: usize,
    mut sink: impl FnMut(&[u8], usize),
) {
    let out = w20a03_bilerp(src, stride, fx, fy, 16, 8);
    sink(&out, 16);
}

/// `0x00a34790`: bilinear predictor for a 32x16 block; the result is handed to
/// `sink(block, stride = 32)` (the retail tail call, callee unresolved).
pub fn w20a03_bilerp_32x16(
    src: &[u8],
    stride: usize,
    fx: usize,
    fy: usize,
    mut sink: impl FnMut(&[u8], usize),
) {
    let out = w20a03_bilerp(src, stride, fx, fy, 32, 16);
    sink(&out, 32);
}

/// `0x00a349b0`: bilinear predictor for a 32x32 block; the result is handed to
/// `sink(block, stride = 32)` (the retail tail call, callee unresolved).
pub fn w20a03_bilerp_32x32(
    src: &[u8],
    stride: usize,
    fx: usize,
    fy: usize,
    mut sink: impl FnMut(&[u8], usize),
) {
    let out = w20a03_bilerp(src, stride, fx, fy, 32, 32);
    sink(&out, 32);
}

/// `0x00a34bd0`: bilinear predictor for a 32x64 block; the result is handed to
/// `sink(block, stride = 32)` (the retail tail call, callee unresolved).
pub fn w20a03_bilerp_32x64(
    src: &[u8],
    stride: usize,
    fx: usize,
    fy: usize,
    mut sink: impl FnMut(&[u8], usize),
) {
    let out = w20a03_bilerp(src, stride, fx, fy, 32, 64);
    sink(&out, 32);
}

/// `0x00a35110`: bilinear predictor for a 64x32 block; the result is handed to
/// `sink(block, stride = 64)` (the retail tail call, callee unresolved).
pub fn w20a03_bilerp_64x32(
    src: &[u8],
    stride: usize,
    fx: usize,
    fy: usize,
    mut sink: impl FnMut(&[u8], usize),
) {
    let out = w20a03_bilerp(src, stride, fx, fy, 64, 32);
    sink(&out, 64);
}

/// `0x00a34df0`: bilinear predictor for a 4x4 block; the result is handed to
/// `sink(block, stride = 4)` (the retail tail call, callee unresolved).
pub fn w20a03_bilerp_4x4(
    src: &[u8],
    stride: usize,
    fx: usize,
    fy: usize,
    mut sink: impl FnMut(&[u8], usize),
) {
    let out = w20a03_bilerp(src, stride, fx, fy, 4, 4);
    sink(&out, 4);
}

/// `0x00a35820`: bilinear predictor for a 8x4 block; the result is handed to
/// `sink(block, stride = 8)` (the retail tail call, callee unresolved).
pub fn w20a03_bilerp_8x4(
    src: &[u8],
    stride: usize,
    fx: usize,
    fy: usize,
    mut sink: impl FnMut(&[u8], usize),
) {
    let out = w20a03_bilerp(src, stride, fx, fy, 8, 4);
    sink(&out, 8);
}

/// `0x0097ac30`: when the field at +0x18 of the object is non-zero, sets the
/// global at DAT 011cb5e4 to 0x91 and calls the (unresolved) dispatcher with
/// three zero args; returns the new global value, or `None` when untouched.
pub fn w20a03_set_state_91(field_18: u32, mut dispatch: impl FnMut(u32, [u32; 3])) -> Option<u32> {
    if field_18 == 0 {
        return None;
    }
    dispatch(0x91, [0, 0, 0]);
    Some(0x91)
}

/// `0x0097b970`: forwards five args to the 11-arg worker: slot 4 is the
/// constant 0x800, slots 6..10 the global quad at 00f3ca88, last slot 0.
pub fn w20a03_forward_args(p: [u32; 5], global_quad: [u32; 4]) -> [u32; 11] {
    [
        p[0],
        p[1],
        p[2],
        p[3],
        0x800,
        p[4],
        global_quad[0],
        global_quad[1],
        global_quad[2],
        global_quad[3],
        0,
    ]
}

/// `0x00b3ada0`: status code for a connection-like object: 6 when the
/// inner call fails, else 8 for mode byte (+0x90) 3, 7 for 4, otherwise 0.
pub fn w20a03_mode_status(inner_ok: bool, mode: u8) -> u8 {
    if !inner_ok {
        return 6;
    }
    match mode {
        3 => 8,
        4 => 7,
        _ => 0,
    }
}

/// `0x00c87c10`: CRT `_allrem`, signed 64-bit remainder (sign follows the
/// dividend), operands passed as (lo, hi) 32-bit halves.  Divisor 0 traps.
pub fn w20a03_allrem(lo1: u32, hi1: u32, lo2: u32, hi2: u32) -> i64 {
    let a = (((hi1 as u64) << 32) | lo1 as u64) as i64;
    let b = (((hi2 as u64) << 32) | lo2 as u64) as i64;
    a.wrapping_rem(b)
}

/// `0x00c9c600`: leap days elapsed before `year`: `(y-1)/4 - 17 +
/// ((y+299)/400 - (y-1)/100)` with C truncating division.
pub fn w20a03_leap_days(year: i32) -> i32 {
    let y1 = year - 1;
    y1 / 4 - 0x11 + ((year + 299) / 400 - y1 / 100)
}

#[cfg(test)]
mod tests_wave20_agent03 {
    use super::*;

    fn img(n: usize) -> Vec<u8> {
        (0..n).map(|i| (i * 7 % 251) as u8).collect()
    }

    #[test]
    fn weights_sum_128() {
        assert!(W20A03_BILERP_WEIGHTS.iter().all(|w| w.0 + w.1 == 128));
    }

    #[test]
    fn zero_fraction_copies() {
        let s = img(100 * 100);
        let o = w20a03_bilerp(&s, 100, 0, 0, 16, 16);
        for r in 0..16 {
            assert_eq!(&o[r * 16..r * 16 + 16], &s[r * 100..r * 100 + 16]);
        }
    }

    #[test]
    fn half_fraction_horizontal() {
        let mut s = vec![0u8; 20 * 5];
        for r in 0..5 {
            for c in 0..20 {
                s[r * 20 + c] = (c * 10) as u8;
            }
        }
        let o = w20a03_bilerp(&s, 20, 4, 0, 4, 4);
        assert_eq!(o[0], 5);
        assert_eq!(o[1], 15);
    }

    #[test]
    fn vertical_rounding() {
        let mut s = vec![0u8; 8 * 5];
        for r in 0..5 {
            for c in 0..8 {
                s[r * 8 + c] = if r == 1 { 1 } else { 0 };
            }
        }
        // fy=4: (0*64 + 1*64 + 64) >> 7 = 1 on row 0
        let o = w20a03_bilerp(&s, 8, 0, 4, 4, 4);
        assert_eq!(o[0], 1);
    }

    #[test]
    fn wrappers_shape() {
        let s = img(200 * 100);
        let cases: [(&str, usize, usize); 9] = [
            ("16x16", 16, 16),
            ("16x32", 16, 32),
            ("16x8", 16, 8),
            ("32x16", 32, 16),
            ("32x32", 32, 32),
            ("32x64", 32, 64),
            ("64x32", 64, 32),
            ("4x4", 4, 4),
            ("8x4", 8, 4),
        ];
        for (name, w, h) in cases {
            let mut got = (0usize, 0usize);
            let mut f = |b: &[u8], st: usize| got = (b.len(), st);
            match name {
                "16x16" => w20a03_bilerp_16x16(&s, 200, 3, 5, &mut f),
                "16x32" => w20a03_bilerp_16x32(&s, 200, 3, 5, &mut f),
                "16x8" => w20a03_bilerp_16x8(&s, 200, 3, 5, &mut f),
                "32x16" => w20a03_bilerp_32x16(&s, 200, 3, 5, &mut f),
                "32x32" => w20a03_bilerp_32x32(&s, 200, 3, 5, &mut f),
                "32x64" => w20a03_bilerp_32x64(&s, 200, 3, 5, &mut f),
                "64x32" => w20a03_bilerp_64x32(&s, 200, 3, 5, &mut f),
                "4x4" => w20a03_bilerp_4x4(&s, 200, 3, 5, &mut f),
                _ => w20a03_bilerp_8x4(&s, 200, 3, 5, &mut f),
            }
            assert_eq!(got, (w * h, w), "{name}");
            let want = w20a03_bilerp(&s, 200, 3, 5, w, h);
            let mut same = false;
            let mut f2 = |b: &[u8], _st: usize| same = b == want.as_slice();
            match name {
                "4x4" => w20a03_bilerp_4x4(&s, 200, 3, 5, &mut f2),
                "64x32" => w20a03_bilerp_64x32(&s, 200, 3, 5, &mut f2),
                _ => same = true,
            }
            assert!(same);
        }
    }

    #[test]
    fn small_helpers() {
        let mut seen = None;
        assert_eq!(w20a03_set_state_91(0, |a, b| seen = Some((a, b))), None);
        assert!(seen.is_none());
        assert_eq!(
            w20a03_set_state_91(1, |a, b| seen = Some((a, b))),
            Some(0x91)
        );
        assert_eq!(seen, Some((0x91, [0, 0, 0])));
        let a = w20a03_forward_args([1, 2, 3, 4, 5], [6, 7, 8, 9]);
        assert_eq!(a, [1, 2, 3, 4, 0x800, 5, 6, 7, 8, 9, 0]);
        assert_eq!(w20a03_mode_status(false, 3), 6);
        assert_eq!(w20a03_mode_status(true, 3), 8);
        assert_eq!(w20a03_mode_status(true, 4), 7);
        assert_eq!(w20a03_mode_status(true, 0), 0);
    }

    #[test]
    fn allrem_and_leap() {
        assert_eq!(w20a03_allrem(7, 0, 3, 0), 1);
        assert_eq!(w20a03_allrem((-7i64) as u32, 0xffff_ffff, 3, 0), -1);
        assert_eq!(w20a03_allrem(7, 0, (-3i64) as u32, 0xffff_ffff), 1);
        assert_eq!(w20a03_allrem(0, 1, 3, 0), 1);
        assert_eq!(w20a03_leap_days(1970), 461);
        assert_eq!(w20a03_leap_days(2000), 1999 / 4 - 17 + (2299 / 400 - 19));
    }
}

// ===========================================================================
// Merged from src/wave21/agent03.rs
// ===========================================================================
// Wave 21 agent 03: dialog flag predicates, colour modulate, scroll clamp.

/// `0x0077ea00`: true when all three provider records are ready
/// (`flag_a` byte, `flag_b` byte, `count` dword at offset 4 of each).
pub fn w21a03_all_providers_ready(flag_a: u8, flag_b: u8, count: u32) -> bool {
    flag_a != 0 && flag_b != 0 && count != 0
}

/// `0x0077f120`: when the three readiness checks pass, forwards to the
/// callee (`forward`, FUN_0077ed70) and returns its value; otherwise 0.
pub fn w21a03_guarded_forward(
    flag_a: u8,
    flag_b: u8,
    count: u32,
    forward: impl FnOnce() -> u32,
) -> u32 {
    if w21a03_all_providers_ready(flag_a, flag_b, count) {
        forward()
    } else {
        0
    }
}

/// `0x00964540`: dialog flag bit 8 (`flags` is the dword at offset 0x30).
pub fn w21a03_dialog_flag_bit8(flags: u32) -> u32 {
    (flags >> 8) & 1
}

/// `0x00964550`: dialog flag bit 9.
pub fn w21a03_dialog_flag_bit9(flags: u32) -> u32 {
    (flags >> 9) & 1
}

/// `0x00964680`: dialog flag bit 7.
pub fn w21a03_dialog_flag_bit7(flags: u32) -> u32 {
    (flags >> 7) & 1
}

/// `0x00964610`: true when none of bits 7..9 (mask `0x380`) are set.
pub fn w21a03_dialog_flags_clear_380(flags: u32) -> bool {
    flags & 0x380 == 0
}

/// `0x00a8f9b0`: component-wise product of two 4-float vectors.
pub fn w21a03_vec4_mul(a: &[f32; 4], b: &[f32; 4]) -> [f32; 4] {
    [a[0] * b[0], a[1] * b[1], a[2] * b[2], a[3] * b[3]]
}

/// Scroll-list state touched by `0x0097a3a0` (offsets 0x64 and 0x67).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W21a03Scroll {
    /// Byte at +0x64: visible row count.
    pub visible: u8,
    /// Byte at +0x67: first visible row.
    pub top: u8,
    /// Child dialog flags (dword +0x30 of the object at +0x50), if any.
    pub child_flags: Option<u32>,
}

/// `0x0097a3a0`: clears child flag bits `0x60000000`, then scrolls so row
/// `row` is visible.  Returns true when the child must be refreshed
/// (FUN_00964f90 in the original).
pub fn w21a03_scroll_to_row(s: &mut W21a03Scroll, row: u8) -> bool {
    if let Some(f) = s.child_flags.as_mut()
        && *f & 0x6000_0000 != 0
    {
        *f &= 0x9fff_ffff;
    }
    let has_child = s.child_flags.is_some();
    if row < s.top {
        s.top = row;
        return has_child;
    }
    if s.visible as u32 + s.top as u32 <= row as u32 && row != 0xff {
        s.top = row.wrapping_sub(s.visible).wrapping_add(1);
        return has_child;
    }
    false
}

#[cfg(test)]
mod tests_wave21_agent03 {
    use super::*;

    #[test]
    fn providers() {
        assert!(w21a03_all_providers_ready(1, 2, 3));
        assert!(!w21a03_all_providers_ready(1, 0, 3));
        assert!(!w21a03_all_providers_ready(1, 1, 0));
        assert_eq!(w21a03_guarded_forward(1, 1, 1, || 9), 9);
        assert_eq!(w21a03_guarded_forward(0, 1, 1, || 9), 0);
    }

    #[test]
    fn flag_bits() {
        assert_eq!(w21a03_dialog_flag_bit8(0x100), 1);
        assert_eq!(w21a03_dialog_flag_bit8(0x200), 0);
        assert_eq!(w21a03_dialog_flag_bit9(0x200), 1);
        assert_eq!(w21a03_dialog_flag_bit7(0x80), 1);
        assert!(w21a03_dialog_flags_clear_380(0x7f));
        assert!(!w21a03_dialog_flags_clear_380(0x100));
    }

    #[test]
    fn vec_mul() {
        assert_eq!(
            w21a03_vec4_mul(&[1.0, 2.0, 3.0, 4.0], &[2.0, 2.0, 0.5, 0.0]),
            [2.0, 4.0, 1.5, 0.0]
        );
    }

    #[test]
    fn scroll() {
        let mut s = W21a03Scroll {
            visible: 4,
            top: 5,
            child_flags: Some(0x6000_0001),
        };
        assert!(w21a03_scroll_to_row(&mut s, 2));
        assert_eq!((s.top, s.child_flags), (2, Some(1)));
        assert!(w21a03_scroll_to_row(&mut s, 7));
        assert_eq!(s.top, 4);
        assert!(!w21a03_scroll_to_row(&mut s, 4));
        assert!(!w21a03_scroll_to_row(&mut s, 0xff));
        s.child_flags = None;
        assert!(!w21a03_scroll_to_row(&mut s, 0));
    }
}
