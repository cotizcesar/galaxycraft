//! Building and resource footprints that block ground paths, with a
//! spatial index for the path search's many box queries.

use super::Rect;

#[derive(Clone, Debug, Default)]
pub struct Obstacles {
    rects: Vec<Rect>,
    /// Spatial index: rects overlapping each `OBSTACLE_BUCKET_PX` square.
    /// A path search asks `blocks_rect` thousands of times; scanning every
    /// building and mineral field per cell made one search take ~38 ms on an
    /// 8-player map.
    buckets: std::collections::HashMap<(i32, i32), Vec<u32>>,
}

const OBSTACLE_BUCKET_PX: i32 = 128;

fn obstacle_buckets(r: &Rect) -> impl Iterator<Item = (i32, i32)> {
    let (x0, x1) = (
        r.left.div_euclid(OBSTACLE_BUCKET_PX),
        r.right.div_euclid(OBSTACLE_BUCKET_PX),
    );
    let (y0, y1) = (
        r.top.div_euclid(OBSTACLE_BUCKET_PX),
        r.bottom.div_euclid(OBSTACLE_BUCKET_PX),
    );
    (y0..=y1).flat_map(move |y| (x0..=x1).map(move |x| (x, y)))
}

impl Obstacles {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.rects.clear();
        self.buckets.clear();
    }

    pub fn push(&mut self, rect: Rect) {
        let k = self.rects.len() as u32;
        for b in obstacle_buckets(&rect) {
            self.buckets.entry(b).or_default().push(k);
        }
        self.rects.push(rect);
    }

    pub fn blocks_point(&self, x: i32, y: i32) -> bool {
        let b = (
            x.div_euclid(OBSTACLE_BUCKET_PX),
            y.div_euclid(OBSTACLE_BUCKET_PX),
        );
        self.buckets.get(&b).is_some_and(|ks| {
            ks.iter()
                .any(|&k| self.rects[k as usize].contains_point(x, y))
        })
    }

    pub fn blocks_rect(&self, rect: &Rect) -> bool {
        obstacle_buckets(rect).any(|b| {
            self.buckets
                .get(&b)
                .is_some_and(|ks| ks.iter().any(|&k| self.rects[k as usize].intersects(rect)))
        })
    }

    pub fn len(&self) -> usize {
        self.rects.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rects.is_empty()
    }
}
