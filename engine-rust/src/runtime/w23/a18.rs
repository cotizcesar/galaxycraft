#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave23/agent18.rs
// ===========================================================================
// Wave 23 agent 18: shared-pointer release loop, deque slot advance and
// vftable-reset destructors of the video decoder family.

/// Shared-pointer control block counters (offset +4 strong, +8 weak).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct W23a18Ctrl {
    pub strong: i32,
    pub weak: i32,
}

/// Outcome of releasing one reference: virtual slot +8 (dispose) and slot
/// +0xc (destroy) invocations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct W23a18Release {
    pub dispose: bool,
    pub destroy: bool,
}

/// Decrements the strong count; when it was 1 dispose runs and the weak
/// count is decremented, destroying when that was 1 (as in the loop body
/// of `0x0071c8e0`).
pub fn w23a18_release_ctrl(c: &mut W23a18Ctrl) -> W23a18Release {
    let mut r = W23a18Release::default();
    let old = c.strong;
    c.strong -= 1;
    if old == 1 {
        r.dispose = true;
        let oldw = c.weak;
        c.weak -= 1;
        if oldw == 1 {
            r.destroy = true;
        }
    }
    r
}

/// `0x0071c8e0`: releases `count` entries of a block deque starting at
/// `head` (8 slots per block, block index masked by `nblocks - 1`).  The
/// slot at `block + 4 + (i & 7) * 8` holds an optional control block.
/// Returns the per-entry release results (None-slots yield default).
pub fn w23a18_release_deque(
    blocks: &mut [Vec<Option<W23a18Ctrl>>],
    head: u32,
    count: u32,
) -> Vec<W23a18Release> {
    let mut out = Vec::new();
    if blocks.is_empty() {
        return out;
    }
    let mask = (blocks.len() as u32).wrapping_sub(1);
    let mut i = head;
    let end = head.wrapping_add(count);
    while i != end {
        let b = ((i >> 3) & mask) as usize;
        let s = (i & 7) as usize;
        let r = match blocks.get_mut(b).and_then(|v| v.get_mut(s)) {
            Some(Some(c)) => w23a18_release_ctrl(c),
            _ => W23a18Release::default(),
        };
        out.push(r);
        i = i.wrapping_add(1);
    }
    out
}

/// Result of the deque back-slot advance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W23a18Slot {
    /// Block map must grow first; carries the new block capacity.
    Grow(u32),
    /// Slot found: (block index, byte offset in block); size incremented.
    Slot(u32, u32),
    /// Block not allocated yet (allocation path).
    Alloc(u32),
}

/// `0x0071e3c0`: deque `emplace_back` slot computation.  `map_len` is
/// field [1], `start` field [2], `size` field [3] (incremented on success);
/// `block_present` tells whether the block pointer is non-null.
pub fn w23a18_deque_back_slot(
    map_len: u32,
    start: u32,
    size: &mut u32,
    block_present: impl Fn(u32) -> bool,
) -> W23a18Slot {
    if (start.wrapping_add(*size) & 7) == 0 && map_len <= (size.wrapping_add(8) >> 3) {
        let mut cap = if map_len != 0 { map_len } else { 8 };
        while cap < map_len + 1 {
            cap = cap.wrapping_mul(2);
        }
        return W23a18Slot::Grow(cap);
    }
    let idx = size.wrapping_add(start);
    *size += 1;
    let block = map_len.wrapping_sub(1) & (idx >> 3);
    if !block_present(block) {
        return W23a18Slot::Alloc(block);
    }
    W23a18Slot::Slot(block, (idx & 7) * 8)
}

/// Vtable tags written by the destructors below.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum W23a18Vtable {
    SharedPtrManager,
    MkvReader,
    VideoDecoder,
}

/// Destructor result: final vtable tag, whether the memory is freed
/// (`param & 1`).
pub type W23a18Dtor = (W23a18Vtable, bool);

/// `0x0071cd30`: shared_ptr_manager scalar destructor.
pub fn w23a18_manager_dtor(flags: u8) -> W23a18Dtor {
    (W23a18Vtable::SharedPtrManager, flags & 1 != 0)
}

/// `0x0071cd60`: IMkvReader scalar destructor.
pub fn w23a18_reader_dtor_a(flags: u8) -> W23a18Dtor {
    (W23a18Vtable::MkvReader, flags & 1 != 0)
}

/// `0x0071cdd0`: IMkvReader scalar destructor (second copy).
pub fn w23a18_reader_dtor_b(flags: u8) -> W23a18Dtor {
    (W23a18Vtable::MkvReader, flags & 1 != 0)
}

/// `0x0071ce00`: VideoDecoder base scalar destructor.
pub fn w23a18_video_dtor(flags: u8) -> W23a18Dtor {
    (W23a18Vtable::VideoDecoder, flags & 1 != 0)
}

/// `0x0071cd90`: MkvDecoder scalar destructor; runs the teardown callback
/// then resets to the VideoDecoder vtable.  Returns the teardown-call flag
/// order via `teardown`.
pub fn w23a18_mkv_dtor(flags: u8, teardown: impl FnOnce()) -> W23a18Dtor {
    teardown();
    (W23a18Vtable::VideoDecoder, flags & 1 != 0)
}

#[cfg(test)]
mod tests_wave23_agent18 {
    use super::*;

    #[test]
    fn release_ctrl() {
        let mut c = W23a18Ctrl { strong: 1, weak: 1 };
        let r = w23a18_release_ctrl(&mut c);
        assert!(r.dispose && r.destroy);
        let mut c = W23a18Ctrl { strong: 2, weak: 1 };
        assert_eq!(w23a18_release_ctrl(&mut c), W23a18Release::default());
        assert_eq!(c.strong, 1);
        let mut c = W23a18Ctrl { strong: 1, weak: 2 };
        let r = w23a18_release_ctrl(&mut c);
        assert!(r.dispose && !r.destroy);
    }

    #[test]
    fn release_deque() {
        let c = Some(W23a18Ctrl { strong: 1, weak: 1 });
        let mut blocks = vec![vec![c, None, c, c, c, c, c, c], vec![c; 8]];
        let r = w23a18_release_deque(&mut blocks, 6, 4);
        assert_eq!(r.len(), 4);
        assert!(r.iter().all(|x| x.destroy));
        assert!(w23a18_release_deque(&mut [], 0, 3).is_empty());
        assert!(!w23a18_release_deque(&mut blocks, 1, 1)[0].dispose);
    }

    #[test]
    fn back_slot() {
        let mut size = 3;
        assert_eq!(
            w23a18_deque_back_slot(2, 0, &mut size, |_| true),
            W23a18Slot::Slot(0, 24)
        );
        assert_eq!(size, 4);
        let mut size = 0;
        assert_eq!(
            w23a18_deque_back_slot(0, 0, &mut size, |_| true),
            W23a18Slot::Grow(8)
        );
        let mut size = 8;
        assert_eq!(
            w23a18_deque_back_slot(1, 0, &mut size, |_| true),
            W23a18Slot::Grow(2)
        );
        let mut size = 1;
        assert_eq!(
            w23a18_deque_back_slot(2, 0, &mut size, |_| false),
            W23a18Slot::Alloc(0)
        );
    }

    #[test]
    fn dtors() {
        assert_eq!(
            w23a18_manager_dtor(1),
            (W23a18Vtable::SharedPtrManager, true)
        );
        assert_eq!(w23a18_reader_dtor_a(2), (W23a18Vtable::MkvReader, false));
        assert_eq!(w23a18_reader_dtor_b(3), (W23a18Vtable::MkvReader, true));
        assert_eq!(w23a18_video_dtor(0), (W23a18Vtable::VideoDecoder, false));
        let mut ran = false;
        assert_eq!(
            w23a18_mkv_dtor(1, || ran = true),
            (W23a18Vtable::VideoDecoder, true)
        );
        assert!(ran);
    }
}
