#![allow(unused_imports)]
use super::super::super::*;

// ===========================================================================
// Merged from src/wave15/agent04.rs
// ===========================================================================
// Wave 15 translation batch 04.
//
// Pure helpers for the object/bullet list code.  Node links are modelled
// with indices (0 / `None` = null pointer).
//
// Skipped (noreturn tails, unmodelled callees or too large):
// FUN_00534160 (huge requirement switch with ~30 unmodelled callees),
// FUN_00534700 (ends in a noreturn call), FUN_00567410 (noreturn branches),
// FUN_00568850 (ends in a noreturn call), FUN_00568920 (bullet state machine,
// SEH and many callees), FUN_00570950 (unmodelled allocator and callees),
// FUN_005852f0 (tail is a noreturn call), FUN_00591b30 (noreturn branches),
// FUN_005c75d0 (noreturn tail), FUN_0063bb34 (noreturn tail, in_EAX arg),
// FUN_006514d0 (noreturn branch), FUN_00785440 (UI glue, many unknown callees).

/// Intrusive list node pair `(link0, link1)` used by `0x005771a0`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W15a04Links {
    /// Word at +0 of the node.
    pub link0: usize,
    /// Word at +4 of the node.
    pub link1: usize,
}

/// `0x005771a0`: inserts `node` right after the head of the list described
/// by `head` / `tail` (both 0 when empty).  `nodes` is indexed by node id,
/// id 0 is the null pointer.  On an empty list both ends become `node`; when
/// the old head was also the tail, the tail becomes `node`.
pub fn w15a04_list_insert_after_head(
    nodes: &mut [W15a04Links],
    head: &mut usize,
    tail: &mut usize,
    node: usize,
) {
    let h = *head;
    if h == 0 {
        *tail = node;
        *head = node;
        return;
    }
    if *tail == h {
        *tail = node;
    }
    nodes[node].link0 = h;
    let next = nodes[h].link1;
    nodes[node].link1 = next;
    if next != 0 {
        nodes[next].link0 = node;
    }
    nodes[h].link1 = node;
}

/// `0x00569170`: absolute angular distance between two 8-bit angles
/// (0..=128).
pub fn w15a04_angle_distance(a: i32, b: i32) -> u32 {
    let d = (b.wrapping_sub(a) & 0xff) as u32;
    if d < 0x81 { d } else { 0x100 - d }
}

/// `0x005692b0`: 8-bit direction from `(x1, y1)` to `(x2, y2)`.  `atan` is
/// the table lookup callee (FUN_00569120) taking `(dy * 256) / dx`.
pub fn w15a04_direction(x1: i32, y1: i32, x2: i32, y2: i32, atan: impl FnOnce(i32) -> i32) -> i32 {
    let dx = x2.wrapping_sub(x1);
    if dx == 0 {
        return if y2.wrapping_sub(y1) < 1 { 0 } else { 0x80 };
    }
    let r = atan(y2.wrapping_sub(y1).wrapping_mul(0x100) / dx);
    if dx >= 0 {
        return r + 0x40;
    }
    let r = r + 0xc0;
    if r == 0x100 { 0 } else { r }
}

/// `0x0057b860`: address of the 0x28-byte record number `idx` (1-based) in
/// the table at `base`; `None` for idx 0.
pub fn w15a04_record_addr(base: u32, idx: u32) -> Option<u32> {
    if idx == 0 {
        None
    } else {
        Some(base.wrapping_add(idx.wrapping_mul(0x28)).wrapping_sub(0x28))
    }
}

/// Child node used by the list walkers below.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W15a04Child {
    /// Next node index (None = null).
    pub next: Option<usize>,
    /// Byte at +10 (type).
    pub kind: u8,
    /// Word at +0xc (flags).
    pub flags: u16,
}

/// `0x0057cc60`: walks the child list starting at `head` and sets flag bit 0
/// on every child whose type byte is 0x0b.
pub fn w15a04_flag_kind11(nodes: &mut [W15a04Child], head: Option<usize>) {
    let mut cur = head;
    while let Some(i) = cur {
        if nodes[i].kind == 0x0b {
            nodes[i].flags |= 1;
        }
        cur = nodes[i].next;
    }
}

/// Node for `0x0057ccd0` (next word at +0, id word at +8, byte at +0x30).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W15a04IdNode {
    /// Next node index (None = null).
    pub next: Option<usize>,
    /// Word at +8.
    pub id: u16,
    /// Byte at +0x30.
    pub value: u8,
}

/// `0x0057ccd0`: if bit 0 of `flags` (+0xe) is set, clears it and scans the
/// list for the first node with id in 0x231..=0x23a, calls `on_found` (the
/// FUN_00571640 callee) and returns that node's byte; otherwise returns 0.
pub fn w15a04_consume_flag_scan(
    flags: &mut u8,
    nodes: &[W15a04IdNode],
    head: Option<usize>,
    mut on_found: impl FnMut(usize),
) -> u8 {
    if *flags & 1 == 0 {
        return 0;
    }
    *flags &= 0xfe;
    let mut cur = head;
    while let Some(i) = cur {
        let n = &nodes[i];
        if n.id > 0x230 && n.id < 0x23b {
            on_found(i);
            return n.value;
        }
        cur = n.next;
    }
    0
}

/// `0x0057d6e0`: stores `new` into `*cur` (+0xc); when the masked value
/// changed and is non-zero, sets flag bit 0 on every child in the list.
pub fn w15a04_set_masked_byte(
    cur: &mut u8,
    new: u8,
    mask: u8,
    nodes: &mut [W15a04Child],
    head: Option<usize>,
) {
    if (*cur & mask) != (new & mask) && (new & mask) != 0 {
        let mut c = head;
        while let Some(i) = c {
            nodes[i].flags |= 1;
            c = nodes[i].next;
        }
    }
    *cur = new;
}

/// `0x005672e0`: gated spawn.  `head` is the free-list head (0 = none),
/// `counter` the live counter, `tick` the value of FUN_0060b530.  `flag`
/// is FUN_005b5ad0's result and `kind_in_set` tells whether `kind` is in the
/// pass-through set; `spawn` is FUN_00567eb0, `relink` runs the two list
/// moves.  Returns the old head or 0.
pub fn w15a04_gated_spawn(
    kind: i32,
    head: u32,
    counter: &mut u32,
    tick: impl FnOnce() -> u32,
    flag: impl FnOnce() -> bool,
    spawn: impl FnOnce() -> bool,
    relink: impl FnOnce(),
) -> u32 {
    if head == 0 || (kind == 0x67 && *counter >= tick()) {
        return 0;
    }
    if flag() && !w15a04_spawn_kind_passes(kind) {
        return 0;
    }
    if spawn() {
        relink();
        *counter += 1;
        return head;
    }
    0
}

/// Switch-case set of `0x005672e0` (kinds allowed when the flag is set).
pub fn w15a04_spawn_kind_passes(kind: i32) -> bool {
    matches!(
        kind,
        6 | 0x20 | 0x21 | 0x22 | 0x32 | 0x33 | 0x36 | 0x38..=0x3d | 0x44 | 0x45
            | 0x52..=0x54 | 0x66 | 0x69..=0x6c
    )
}

#[cfg(test)]
mod tests_wave15_agent04 {
    use super::*;

    #[test]
    fn angle_dist() {
        assert_eq!(w15a04_angle_distance(0, 0x80), 0x80);
        assert_eq!(w15a04_angle_distance(10, 5), 5);
        assert_eq!(w15a04_angle_distance(0, 0xf0), 0x10);
    }

    #[test]
    fn direction() {
        assert_eq!(w15a04_direction(0, 0, 0, 5, |_| 0), 0x80);
        assert_eq!(w15a04_direction(0, 0, 0, 0, |_| 0), 0);
        assert_eq!(w15a04_direction(0, 0, 4, 0, |v| v), 0x40);
        assert_eq!(w15a04_direction(4, 0, 0, 0, |_| 0x40), 0);
        assert_eq!(w15a04_direction(4, 0, 0, 0, |_| 0), 0xc0);
    }

    #[test]
    fn record() {
        assert_eq!(w15a04_record_addr(1000, 0), None);
        assert_eq!(w15a04_record_addr(1000, 1), Some(1000));
        assert_eq!(w15a04_record_addr(1000, 3), Some(1000 + 0x50));
    }

    #[test]
    fn insert() {
        let mut n = [W15a04Links::default(); 4];
        let (mut h, mut t) = (0, 0);
        w15a04_list_insert_after_head(&mut n, &mut h, &mut t, 1);
        assert_eq!((h, t), (1, 1));
        w15a04_list_insert_after_head(&mut n, &mut h, &mut t, 2);
        assert_eq!((h, t), (1, 2));
        assert_eq!(n[1].link1, 2);
        assert_eq!(n[2].link0, 1);
        w15a04_list_insert_after_head(&mut n, &mut h, &mut t, 3);
        assert_eq!(n[1].link1, 3);
        assert_eq!(n[3].link1, 2);
        assert_eq!(n[2].link0, 3);
        assert_eq!(t, 2);
    }

    #[test]
    fn flags() {
        let mut n = [
            W15a04Child {
                next: Some(1),
                kind: 11,
                flags: 0,
            },
            W15a04Child {
                next: None,
                kind: 3,
                flags: 0,
            },
        ];
        w15a04_flag_kind11(&mut n, Some(0));
        assert_eq!((n[0].flags, n[1].flags), (1, 0));
        let mut cur = 0x01u8;
        w15a04_set_masked_byte(&mut cur, 0x02, 0x03, &mut n, Some(0));
        assert_eq!((cur, n[1].flags), (2, 1));
        let mut cur = 0x01u8;
        n[1].flags = 0;
        w15a04_set_masked_byte(&mut cur, 0x04, 0x03, &mut n, Some(0));
        assert_eq!(n[1].flags, 0);
    }

    #[test]
    fn scan() {
        let nodes = [
            W15a04IdNode {
                next: Some(1),
                id: 0x230,
                value: 1,
            },
            W15a04IdNode {
                next: None,
                id: 0x231,
                value: 7,
            },
        ];
        let mut f = 1u8;
        let mut hit = None;
        assert_eq!(
            w15a04_consume_flag_scan(&mut f, &nodes, Some(0), |i| hit = Some(i)),
            7
        );
        assert_eq!((f, hit), (0, Some(1)));
        assert_eq!(w15a04_consume_flag_scan(&mut f, &nodes, Some(0), |_| {}), 0);
    }

    #[test]
    fn spawn() {
        let mut c = 0u32;
        assert_eq!(
            w15a04_gated_spawn(6, 9, &mut c, || 0, || true, || true, || {}),
            9
        );
        assert_eq!(c, 1);
        assert_eq!(
            w15a04_gated_spawn(7, 9, &mut c, || 0, || true, || true, || {}),
            0
        );
        assert_eq!(
            w15a04_gated_spawn(7, 0, &mut c, || 0, || false, || true, || {}),
            0
        );
        assert_eq!(
            w15a04_gated_spawn(0x67, 9, &mut c, || 1, || false, || true, || {}),
            0
        );
        assert_eq!(
            w15a04_gated_spawn(0x67, 9, &mut c, || 5, || false, || false, || {}),
            0
        );
    }
}
