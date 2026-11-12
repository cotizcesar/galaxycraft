#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave22/agent24.rs
// ===========================================================================
// Wave 22 agent 24: list-header reset and slot-init helpers.

/// Result of resetting a circular list header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W22a24Reset {
    /// The list was already empty (old `+4` link pointed at the header).
    Empty,
    /// Old first link; `node_ptr` is the value at `link + 8`.
    HadNode { link: u32, node_ptr: u32 },
}

/// `0x006ad170`: resets a circular list header at `self_addr` (words
/// `[0]=[1]=self_addr`, `[2]=0`) and reports the previous `+4` link.
/// `node_of_link` reads the word at `link + 8`.
pub fn w22a24_reset_list_header(
    header: &mut [u32; 3],
    self_addr: u32,
    node_of_link: impl Fn(u32) -> u32,
) -> W22a24Reset {
    let old = header[1];
    header[1] = self_addr;
    header[0] = self_addr;
    header[2] = 0;
    if old == self_addr {
        return W22a24Reset::Empty;
    }
    W22a24Reset::HadNode {
        link: old,
        node_ptr: node_of_link(old),
    }
}

/// Buffers to release for one node in `0x006ad1c0`, in release order,
/// followed by the node itself (size `0x170`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct W22a24NodeFrees {
    pub buffers: Vec<u32>,
    pub node: u32,
    pub node_size: u32,
}

/// `0x006ad1c0`: resets the header like `0x006ad170`, then (when a node
/// exists) frees the non-null pointers at node offsets `0x16c`, `0x164`,
/// `0x15c`, then the node (size `0x170`).  `None` when the list was empty
/// or the node pointer is null (the original calls a non-returning
/// helper in the latter case).
pub fn w22a24_destroy_list_plan(
    header: &mut [u32; 3],
    self_addr: u32,
    node_of_link: impl Fn(u32) -> u32,
    read_u32: impl Fn(u32, u32) -> u32,
) -> Option<W22a24NodeFrees> {
    match w22a24_reset_list_header(header, self_addr, node_of_link) {
        W22a24Reset::Empty => None,
        W22a24Reset::HadNode { node_ptr, .. } => {
            if node_ptr == 0 {
                return None;
            }
            let buffers = [0x16cu32, 0x164, 0x15c]
                .iter()
                .map(|&o| read_u32(node_ptr, o))
                .filter(|&p| p != 0)
                .collect();
            Some(W22a24NodeFrees {
                buffers,
                node: node_ptr,
                node_size: 0x170,
            })
        }
    }
}

/// Fields written by `0x006acd50`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct W22a24SlotInit {
    /// `param_6[1]`
    pub f1: i32,
    /// `param_6[2]` (starts at 1, `| 0x400` when `flag`)
    pub flags: u32,
    /// `param_6[3]`
    pub f3: i32,
    /// `param_6[5]`
    pub f5: i32,
}

/// `0x006acd50`: initialises a slot record.  Returns `None` for the two
/// non-returning error paths (`reject` nonzero, or missing record).
/// `alloc(param_3)` models `FUN_006b1450(param_3, 0, 1)`; the result is
/// also stored at `owner[0x18]` (returned as the second tuple item).
/// The `bool` is true when the virtual call at vtable `+0x10` is made.
pub fn w22a24_slot_init(
    param2: i32,
    param3: i32,
    set_flag: bool,
    reject: bool,
    has_record: bool,
    alloc: impl Fn(i32) -> i32,
) -> Option<(W22a24SlotInit, i32, bool)> {
    if reject || !has_record {
        return None;
    }
    let h = alloc(param3);
    let mut flags = 1u32;
    if set_flag {
        flags |= 0x400;
    }
    Some((
        W22a24SlotInit {
            f1: h,
            flags,
            f3: param2,
            f5: param3,
        },
        h,
        set_flag,
    ))
}

#[cfg(test)]
mod tests_wave22_agent24 {
    use super::*;

    #[test]
    fn reset_header() {
        let mut h = [1, 7, 9];
        assert_eq!(
            w22a24_reset_list_header(&mut h, 100, |l| l + 1),
            W22a24Reset::HadNode {
                link: 7,
                node_ptr: 8
            }
        );
        assert_eq!(h, [100, 100, 0]);
        assert_eq!(
            w22a24_reset_list_header(&mut h, 100, |l| l),
            W22a24Reset::Empty
        );
    }

    #[test]
    fn destroy_plan() {
        let mut h = [0, 5, 0];
        let p = w22a24_destroy_list_plan(&mut h, 1, |_| 50, |_, o| if o == 0x164 { 0 } else { o })
            .unwrap();
        assert_eq!(p.buffers, vec![0x16c, 0x15c]);
        assert_eq!((p.node, p.node_size), (50, 0x170));
        let mut h = [0, 5, 0];
        assert!(w22a24_destroy_list_plan(&mut h, 1, |_| 0, |_, _| 1).is_none());
        let mut h = [0, 1, 0];
        assert!(w22a24_destroy_list_plan(&mut h, 1, |_| 3, |_, _| 1).is_none());
    }

    #[test]
    fn slot_init() {
        let (s, h, v) = w22a24_slot_init(3, 9, true, false, true, |x| x * 2).unwrap();
        assert_eq!(
            (s.f1, s.flags, s.f3, s.f5, h, v),
            (18, 0x401, 3, 9, 18, true)
        );
        let (s, _, v) = w22a24_slot_init(3, 9, false, false, true, |x| x).unwrap();
        assert_eq!((s.flags, v), (1, false));
        assert!(w22a24_slot_init(0, 0, false, true, true, |x| x).is_none());
        assert!(w22a24_slot_init(0, 0, false, false, false, |x| x).is_none());
    }
}
