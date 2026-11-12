#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent28.rs
// ===========================================================================
// Wave 22 agent 28: intrusive-list helpers, hash buckets and chunk windows.

/// `0x006bcbc0`: clears the bits of `mask` in the flags dword at `+0xc`.
pub fn w22a28_clear_flags(flags: u32, mask: u32) -> u32 {
    flags & !mask
}

/// Retail list node: `next` at `+4`; a link with bit 0 set is the end marker.
/// Here `next`/`head` are `Option<usize>` indices (None = null or tagged end).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W22a28Node {
    /// Byte at `+8`.
    pub key8: u8,
    /// Byte at `+9`.
    pub key9: u8,
    /// Flags byte at `+0x94`.
    pub flags: u8,
    /// Byte at `+0x9c`.
    pub b9c: u8,
    /// Byte at `+0x9d`.
    pub b9d: u8,
    /// Next node (`+4`).
    pub next: Option<usize>,
}

/// `0x006bcbd0`: walks the list from `head`, calls `on_match` on the first
/// node whose `+8` byte equals `key` and returns its index.
pub fn w22a28_find_key8(
    nodes: &[W22a28Node],
    head: Option<usize>,
    key: u8,
    mut on_match: impl FnMut(usize),
) -> Option<usize> {
    let mut cur = head;
    while let Some(i) = cur {
        // next is read before the compare, as in retail
        let next = nodes[i].next;
        if nodes[i].key8 == key {
            on_match(i);
            return Some(i);
        }
        cur = next;
    }
    None
}

/// `0x006bcf90`: pass 1 calls `release` (`FUN_006bc750`) on the first node
/// whose `+9` byte is `player`; pass 2 clears flag 4 (if `+0x9d == player`)
/// and flag 2 (if `+0x9c == player`) on every node, then calls `refresh`
/// (`FUN_006bcdb0`) on each.
pub fn w22a28_release_player(
    nodes: &mut [W22a28Node],
    head: Option<usize>,
    player: u8,
    mut release: impl FnMut(usize),
    mut refresh: impl FnMut(usize),
) {
    let mut cur = head;
    while let Some(i) = cur {
        if nodes[i].key9 == player {
            release(i);
            break;
        }
        cur = nodes[i].next;
    }
    let mut cur = head;
    while let Some(i) = cur {
        let next = nodes[i].next;
        let mut f = nodes[i].flags;
        if f & 4 != 0 && nodes[i].b9d == player {
            f &= 0xfb;
            nodes[i].flags = f;
        }
        if f & 2 != 0 && nodes[i].b9c == player {
            nodes[i].flags = f & 0xfd;
        }
        refresh(i);
        cur = next;
    }
}

/// Transfer record fields used by `0x006bde60` / `0x006be220`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct W22a28Transfer {
    /// `+0x0c` total length.
    pub total: u32,
    /// `+0x94` flags.
    pub flags: u8,
    /// `+0x98` bytes already sent.
    pub sent: u32,
    /// `+0x9d` owner.
    pub owner: u8,
    /// `+0xa0` received watermark.
    pub received: u32,
    /// `+0xa4` data buffer.
    pub buf: Vec<u8>,
}

/// `0x006bde60`: handles a chunk packet (`[?, a, b, off:u32, len:u16, data]`,
/// header 9 bytes).  Returns 0 always.  `lookup(a, b)` is `FUN_006bd0c0`;
/// `reject(player, idx)` models `FUN_006be310` + `FUN_006bcdb0`.
pub fn w22a28_handle_chunk(
    pkt: &[u8],
    size: u32,
    player: u8,
    recs: &mut [W22a28Transfer],
    lookup: impl Fn(u8, u8) -> Option<usize>,
    mut reject: impl FnMut(u8, usize),
) -> u32 {
    if size <= 8 || pkt.len() < 9 {
        return 0;
    }
    let len = u16::from_le_bytes([pkt[7], pkt[8]]) as u32;
    if size != len + 9 {
        return 0;
    }
    let Some(i) = lookup(pkt[1], pkt[2]) else {
        return 0;
    };
    let off = u32::from_le_bytes([pkt[3], pkt[4], pkt[5], pkt[6]]);
    let r = &mut recs[i];
    if off != r.total && off + len <= r.total && off == r.received {
        r.flags |= 4;
        r.owner = player;
        let (o, l) = (off as usize, len as usize);
        if r.buf.len() >= o + l && pkt.len() >= 9 + l {
            r.buf[o..o + l].copy_from_slice(&pkt[9..9 + l]);
        }
    } else {
        reject(player, i);
    }
    0
}

/// `0x006be220`: chunk window for sending (max 0x200).  `enabled` is the
/// global gate.  Returns `(start, len)` of the bytes to copy and may clear
/// flag 2 when the watermark is exceeded.
pub fn w22a28_next_send_window(
    enabled: bool,
    r: &mut W22a28Transfer,
    want: u32,
) -> Option<(u32, u32)> {
    if !enabled {
        return None;
    }
    let mut start = r.sent;
    let want = want.min(0x200);
    if start >= r.total {
        return None;
    }
    let mut len = if want + start <= r.total {
        want
    } else {
        r.total - start
    };
    if r.received < len + start {
        r.flags &= 0xfd;
        len = 0;
        start = r.received;
    }
    Some((start, len))
}

/// Bucket index for a one-byte key in `0x006bee50` / `0x006befd0`
/// (murmur-style finalizer, `buckets` a power of two).
pub fn w22a28_byte_bucket(key: u8, buckets: u32) -> u32 {
    let mut u = (key as u32).wrapping_mul(0x85eb_ca6b);
    u = (u >> 13 ^ u).wrapping_mul(0xc2b2_ae35);
    (u >> 16 ^ u) & buckets.wrapping_sub(1)
}

/// `0x006befd0`: looks `key` up in chained buckets (`chains[b]` = keys in
/// order).  Returns `(bucket, position)`; `None` is the retail not-found
/// path (non-returning throw).  Empty `chains` models the lazy rehash call.
pub fn w22a28_bucket_find(chains: &[Vec<u8>], key: u8) -> Option<(usize, usize)> {
    if chains.is_empty() {
        return None;
    }
    let b = w22a28_byte_bucket(key, chains.len() as u32) as usize;
    chains[b].iter().position(|&k| k == key).map(|p| (b, p))
}

/// `0x006bee50`: checks that each key (stride 0x20 bytes in retail, here an
/// iterator) is present; returns the index of the first missing key.
pub fn w22a28_all_present(chains: &[Vec<u8>], keys: &[u8]) -> Option<usize> {
    keys.iter()
        .position(|&k| w22a28_bucket_find(chains, k).is_none())
}

/// `0x006bf3d0`: dispatches on the global mode flag (`DAT_011a4580`).
pub fn w22a28_mode_dispatch<T>(
    mode: bool,
    arg: u32,
    mut a: impl FnMut(u32) -> T,
    mut b: impl FnMut(u32) -> T,
) -> T {
    if mode { a(arg) } else { b(arg) }
}

#[cfg(test)]
mod tests_wave22_agent28 {
    use super::*;

    fn list() -> Vec<W22a28Node> {
        (0..3)
            .map(|i| W22a28Node {
                key8: i as u8,
                key9: 5,
                flags: 6,
                b9c: 5,
                b9d: 5,
                next: if i < 2 { Some(i + 1) } else { None },
            })
            .collect()
    }

    #[test]
    fn flags_and_find() {
        assert_eq!(w22a28_clear_flags(0xff, 0x0f), 0xf0);
        let n = list();
        let mut hit = 0;
        assert_eq!(w22a28_find_key8(&n, Some(0), 2, |i| hit = i), Some(2));
        assert_eq!(hit, 2);
        assert_eq!(w22a28_find_key8(&n, Some(0), 9, |_| {}), None);
    }

    #[test]
    fn release() {
        let mut n = list();
        let (mut rel, mut cnt) = (vec![], 0);
        w22a28_release_player(&mut n, Some(0), 5, |i| rel.push(i), |_| cnt += 1);
        assert_eq!(rel, vec![0]);
        assert_eq!(cnt, 3);
        assert!(n.iter().all(|x| x.flags == 0));
    }

    #[test]
    fn chunk() {
        let mut recs = vec![W22a28Transfer {
            total: 10,
            buf: vec![0; 10],
            ..Default::default()
        }];
        let mut pkt = vec![0, 0, 0, 0, 0, 0, 0, 2, 0];
        pkt.extend_from_slice(&[7, 8]);
        let mut rej = 0;
        w22a28_handle_chunk(&pkt, 11, 3, &mut recs, |_, _| Some(0), |_, _| rej += 1);
        assert_eq!(
            (recs[0].flags, recs[0].owner, &recs[0].buf[..2]),
            (4, 3, &[7, 8][..])
        );
        assert_eq!(rej, 0);
        recs[0].received = 99;
        w22a28_handle_chunk(&pkt, 11, 3, &mut recs, |_, _| Some(0), |_, _| rej += 1);
        assert_eq!(rej, 1);
        assert_eq!(
            w22a28_handle_chunk(&pkt, 12, 3, &mut recs, |_, _| Some(0), |_, _| rej += 1),
            0
        );
        assert_eq!(rej, 1);
    }

    #[test]
    fn window() {
        let mut r = W22a28Transfer {
            total: 1000,
            received: 600,
            flags: 2,
            ..Default::default()
        };
        assert_eq!(
            w22a28_next_send_window(true, &mut r, 0x1000),
            Some((0, 0x200))
        );
        r.sent = 500;
        assert_eq!(
            w22a28_next_send_window(true, &mut r, 0x1000),
            Some((600, 0))
        );
        assert_eq!(r.flags, 0);
        assert_eq!(w22a28_next_send_window(false, &mut r, 1), None);
        r.sent = 1000;
        assert_eq!(w22a28_next_send_window(true, &mut r, 1), None);
    }

    #[test]
    fn buckets() {
        assert_eq!(w22a28_byte_bucket(0, 8), 0);
        let mut chains = vec![vec![]; 8];
        let b = w22a28_byte_bucket(7, 8) as usize;
        chains[b].push(7);
        assert_eq!(w22a28_bucket_find(&chains, 7), Some((b, 0)));
        assert_eq!(w22a28_bucket_find(&chains, 9), None);
        assert_eq!(w22a28_all_present(&chains, &[7, 9]), Some(1));
        assert_eq!(w22a28_mode_dispatch(true, 1, |x| x + 1, |x| x + 2), 2);
        assert_eq!(w22a28_mode_dispatch(false, 1, |x| x + 1, |x| x + 2), 3);
    }
}
