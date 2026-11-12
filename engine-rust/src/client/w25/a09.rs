#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent09.rs
// ===========================================================================
// Wave 25 agent 09: small UI widget helpers (client module).

/// Widget node used by the w25a09 helpers (indices instead of pointers).
#[derive(Clone, Debug, Default)]
pub struct W25a09Widget {
    /// `+0x00` next sibling.
    pub next: Option<usize>,
    /// `+0x04`, `+0x06`, `+0x08`, `+0x0a`: left, top, right, bottom.
    pub rect: [i16; 4],
    /// `+0x3c` short; when nonzero `+0x4c` anchor is used.
    pub anchored: u16,
    /// `+0x4c` anchor widget.
    pub anchor: Option<usize>,
    /// `+0x30` flag word (bit 5 = already-top).
    pub flags: u32,
    /// `+0x5c` next link used by the walker when `anchored == 0`.
    pub link: Option<usize>,
    /// `+0x5c` first child (hit test).
    pub first_child: Option<usize>,
}

/// `0x00940bf0`: builds the 8-dword invalidate record `{0, 0x80, x, y, x, y,
/// x + w, y + h}` when the flag at the global is set; otherwise `None`.
pub fn w25a09_build_rect_record(enabled: bool, x: i32, y: i32, w: i32, h: i32) -> Option<[i32; 8]> {
    if !enabled {
        return None;
    }
    Some([0, 0x80, x, y, x, y, x.wrapping_add(w), y.wrapping_add(h)])
}

/// `0x00963790`: walks a widget chain, reading the successor before invoking
/// `f` (successor is `next` when `anchored != 0`, else `link`).
pub fn w25a09_walk_chain(nodes: &[W25a09Widget], start: Option<usize>, mut f: impl FnMut(usize)) {
    let mut cur = start;
    while let Some(i) = cur {
        let n = &nodes[i];
        let nxt = if n.anchored != 0 { n.next } else { n.link };
        f(i);
        cur = nxt;
    }
}

/// `0x00964170`: mouse hit test.  Returns the deepest matching child (a
/// later match replaces the current one unless the current has flag bit 5),
/// else the widget itself when its handler accepts, else `None`.
/// `handler(idx)` models the virtual call at `+0x44` with the event.
pub fn w25a09_hit_test(
    nodes: &[W25a09Widget],
    root: usize,
    x: i16,
    y: i16,
    mut handler: impl FnMut(usize) -> bool,
) -> Option<usize> {
    let r = &nodes[root];
    let inside = |n: &W25a09Widget, px: i16, py: i16| {
        n.rect[0] <= px && px <= n.rect[2] && n.rect[1] <= py && py <= n.rect[3]
    };
    if !inside(r, x, y) {
        return None;
    }
    let base = if r.anchored != 0 {
        r.anchor.map(|a| &nodes[a]).unwrap_or(r)
    } else {
        r
    };
    let lx = x.wrapping_sub(base.rect[0]);
    let ly = y.wrapping_sub(base.rect[1]);
    let mut best: Option<usize> = None;
    let mut c = r.first_child;
    while let Some(i) = c {
        let n = &nodes[i];
        if inside(n, lx, ly) && handler(i) && best.is_none_or(|b| nodes[b].flags >> 5 & 1 == 0) {
            best = Some(i);
        }
        c = n.next;
    }
    if best.is_some() {
        return best;
    }
    handler(root).then_some(root)
}

#[cfg(test)]
mod tests_wave25_agent09 {
    use super::*;

    #[test]
    fn rect_record() {
        assert_eq!(w25a09_build_rect_record(false, 1, 2, 3, 4), None);
        assert_eq!(
            w25a09_build_rect_record(true, 1, 2, 3, 4),
            Some([0, 0x80, 1, 2, 1, 2, 4, 6])
        );
    }

    fn tree() -> Vec<W25a09Widget> {
        vec![
            W25a09Widget {
                rect: [0, 0, 100, 100],
                first_child: Some(1),
                ..Default::default()
            },
            W25a09Widget {
                rect: [0, 0, 50, 50],
                next: Some(2),
                ..Default::default()
            },
            W25a09Widget {
                rect: [10, 10, 60, 60],
                ..Default::default()
            },
        ]
    }

    #[test]
    fn hit() {
        let t = tree();
        assert_eq!(w25a09_hit_test(&t, 0, 20, 20, |_| true), Some(2));
        assert_eq!(w25a09_hit_test(&t, 0, 5, 5, |_| true), Some(1));
        assert_eq!(w25a09_hit_test(&t, 0, 90, 90, |i| i == 0), Some(0));
        assert_eq!(w25a09_hit_test(&t, 0, 90, 90, |_| false), None);
        assert_eq!(w25a09_hit_test(&t, 0, 200, 5, |_| true), None);
        let mut t2 = tree();
        t2[1].flags = 0x20;
        assert_eq!(w25a09_hit_test(&t2, 0, 20, 20, |_| true), Some(1));
    }

    #[test]
    fn walk() {
        let mut t = vec![W25a09Widget::default(); 3];
        t[0].link = Some(2);
        t[2].anchored = 1;
        t[2].next = Some(1);
        let mut seen = vec![];
        w25a09_walk_chain(&t, Some(0), |i| seen.push(i));
        assert_eq!(seen, vec![0, 2, 1]);
    }
}
