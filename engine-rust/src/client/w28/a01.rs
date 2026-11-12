#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave28/agent01.rs
// ===========================================================================
// Wave 28 translation batch 01.
//
// Colour/vector helpers, a murmur-style hash bucket lookup and a
// name-copy helper from the client UI area.

/// Result of the bounded name copy.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum W28a01CopyResult {
    /// Copied (code 0).
    Ok,
    /// Destination missing (`0xffffec77`).
    NoDest,
    /// Owner id not found (`0xffffec75`).
    NotFound,
    /// Name does not fit; the retail code takes a non-returning error path.
    TooLong,
}

/// `0x009e9c30`: finds the node whose id equals `owner` and copies its
/// NUL-terminated name into `dest` when `strlen < cap`.  Nodes are
/// `(id, next, name)`; `next` is an index into `nodes`.
pub fn w28a01_copy_owner_name(
    nodes: &[(i32, Option<usize>, Vec<u8>)],
    head: Option<usize>,
    owner: i32,
    dest: Option<&mut Vec<u8>>,
    cap: u32,
) -> W28a01CopyResult {
    let Some(dest) = dest else {
        return W28a01CopyResult::NoDest;
    };
    let mut cur = head;
    let node = loop {
        match cur {
            None => return W28a01CopyResult::NotFound,
            Some(i) if nodes[i].0 == owner => break &nodes[i],
            Some(i) => cur = nodes[i].1,
        }
    };
    let len = node.2.iter().position(|&b| b == 0).unwrap_or(node.2.len());
    if (len as u32) < cap {
        dest.clear();
        dest.extend_from_slice(&node.2[..len]);
        dest.push(0);
        W28a01CopyResult::Ok
    } else {
        W28a01CopyResult::TooLong
    }
}

/// `0x00a8f9d0`: returns the 4-float vector scaled by `s`.
pub fn w28a01_vec4_scale(v: [f32; 4], s: f32) -> [f32; 4] {
    [s * v[0], s * v[1], s * v[2], s * v[3]]
}

/// `0x00a8fa00`: component-wise sum of two 4-float vectors.
pub fn w28a01_vec4_add(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2], a[3] + b[3]]
}

/// `0x00a8fb10`: builds a float colour from four bytes (each / 255.0).
pub fn w28a01_color_from_bytes(r: u8, g: u8, b: u8, a: u8) -> [f32; 4] {
    [
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        a as f32 / 255.0,
    ]
}

/// `0x00a8fbe0`: stores four values into a 4-element record.
pub fn w28a01_vec4_set<T: Copy>(a: T, b: T, c: T, d: T) -> [T; 4] {
    [a, b, c, d]
}

/// `0x00a94110`: murmur3 finaliser hash of `key` masked by `count - 1`.
pub fn w28a01_hash_bucket(key: u32, count: u32) -> u32 {
    let mut h = (key >> 16 ^ key).wrapping_mul(0x85eb_ca6b);
    h = (h >> 13 ^ h).wrapping_mul(0xc2b2_ae35);
    (h >> 16 ^ h) & count.wrapping_sub(1)
}

/// `0x00a94110`: looks `key` up in a chained hash table.  `buckets` holds
/// the head node of each chain; `nodes` is `(next, key)`.  Returns the
/// matching node index (payload at +8), or `None` (the retail code then
/// takes a non-returning error path; an empty table does likewise).
pub fn w28a01_hash_find(
    buckets: &[Option<usize>],
    nodes: &[(Option<usize>, u32)],
    key: u32,
) -> Option<usize> {
    if buckets.is_empty() {
        return None;
    }
    let b = w28a01_hash_bucket(key, buckets.len() as u32) as usize;
    let mut cur = buckets[b];
    while let Some(i) = cur {
        if nodes[i].1 == key {
            return Some(i);
        }
        cur = nodes[i].0;
    }
    None
}

/// `0x00a8a1b0`: radar-style falloff.  `x, y, radius, value` come from the
/// 44-byte record (`[0]`, `[1]`, `[6]`, `[7]`); `scale = s1 * s2` is the
/// product of the two scalar callees; `half_x`/`half_y` are the screen
/// extent globals.  Returns `value * clamp((radius - dist) / radius)`.
pub fn w28a01_falloff(
    x: f32,
    y: f32,
    radius: f32,
    value: f32,
    scale: f32,
    half_x: f32,
    half_y: f32,
) -> f32 {
    if !(radius > 0.0 && value > 0.0) {
        return 0.0;
    }
    let ex = half_x * scale * 0.5;
    let ey = half_y * scale * 0.5;
    let dx = ((x - ex).abs() - ex).max(0.0);
    let dy = ((y - ey).abs() - ey).max(0.0);
    let d = ((dy * dy + dx * dx) as f64).sqrt() as f32;
    let mut t = radius;
    if radius - d <= radius {
        t = radius - d;
    }
    let t = t.max(0.0);
    (t / radius) * (value - 0.0) + 0.0
}

#[cfg(test)]
mod tests_wave28_agent01 {
    use super::*;

    #[test]
    fn copy_name() {
        let nodes = vec![
            (1, Some(1), b"ab\0".to_vec()),
            (2, None, b"hello\0".to_vec()),
        ];
        let mut d = Vec::new();
        assert_eq!(
            w28a01_copy_owner_name(&nodes, Some(0), 2, Some(&mut d), 6),
            W28a01CopyResult::Ok
        );
        assert_eq!(d, b"hello\0");
        assert_eq!(
            w28a01_copy_owner_name(&nodes, Some(0), 2, Some(&mut d), 5),
            W28a01CopyResult::TooLong
        );
        assert_eq!(
            w28a01_copy_owner_name(&nodes, Some(0), 9, Some(&mut d), 5),
            W28a01CopyResult::NotFound
        );
        assert_eq!(
            w28a01_copy_owner_name(&nodes, Some(0), 1, None, 5),
            W28a01CopyResult::NoDest
        );
    }

    #[test]
    fn vec_helpers() {
        assert_eq!(
            w28a01_vec4_scale([1.0, 2.0, 3.0, 4.0], 2.0),
            [2.0, 4.0, 6.0, 8.0]
        );
        assert_eq!(w28a01_vec4_add([1.0; 4], [2.0; 4]), [3.0; 4]);
        assert_eq!(
            w28a01_color_from_bytes(255, 0, 51, 255),
            [1.0, 0.0, 0.2, 1.0]
        );
        assert_eq!(w28a01_vec4_set(1, 2, 3, 4), [1, 2, 3, 4]);
    }

    #[test]
    fn hash_lookup() {
        assert_eq!(w28a01_hash_bucket(0, 8), 0);
        let mut buckets = vec![None; 8];
        let mut nodes = Vec::new();
        for key in [5u32, 77, 1234] {
            let b = w28a01_hash_bucket(key, 8) as usize;
            nodes.push((buckets[b], key));
            buckets[b] = Some(nodes.len() - 1);
        }
        assert_eq!(w28a01_hash_find(&buckets, &nodes, 77), Some(1));
        assert_eq!(w28a01_hash_find(&buckets, &nodes, 78), None);
        assert_eq!(w28a01_hash_find(&[], &nodes, 5), None);
    }

    #[test]
    fn falloff() {
        assert_eq!(w28a01_falloff(0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0), 0.0);
        // inside the extent: distance 0, full value
        assert_eq!(w28a01_falloff(0.5, 0.5, 10.0, 4.0, 1.0, 2.0, 2.0), 4.0);
        // far away: zero
        assert_eq!(w28a01_falloff(100.0, 0.0, 10.0, 4.0, 1.0, 2.0, 2.0), 0.0);
    }
}
