#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave19/agent01.rs
// ===========================================================================
// Wave 19 translation batch 01: chained hash-table walks, a record-stream
// reader and the dialog-proc decision tables of the retail client.

/// `0x005499c0`: reads exactly one `0x7c`-byte record through the stream
/// vtable `read(dst_offset, want)` (slot 1).  Short reads are retried with the
/// remaining byte count until the running total is a multiple of `0x7c`; a
/// zero-byte read (or total below `0x7c`) is the "Premature end of stream"
/// error.  Returns the number of bytes accumulated on success (>= `0x7c`).
pub fn w19a01_read_record(
    mut read: impl FnMut(usize, usize) -> usize,
) -> Result<usize, &'static str> {
    const REC: usize = 0x7c;
    let mut got = read(0, REC);
    while got != (got / REC) * REC {
        let n = read(got, REC - got);
        if n == 0 {
            return Err("Premature end of stream");
        }
        got += n;
    }
    if got < REC {
        return Err("Premature end of stream");
    }
    Ok(got)
}

/// `0x0054a0e0`: chained hash lookup.  Bucket = `hash & (bucket_count - 1)`;
/// the chain is walked (`next` gives a node's successor, `eq` compares the
/// key stored at node+4) and the first equal node is returned (its value
/// lives at node+0x20).  Retail aborts when the key is missing; that is
/// `None` here.
pub fn w19a01_hash_find(
    bucket_count: u32,
    hash: u32,
    buckets: &[Option<usize>],
    next: impl Fn(usize) -> Option<usize>,
    eq: impl Fn(usize) -> bool,
) -> Option<usize> {
    let slot = (bucket_count.wrapping_sub(1) & hash) as usize;
    let mut cur = *buckets.get(slot)?;
    while let Some(n) = cur {
        if eq(n) {
            return Some(n);
        }
        cur = next(n);
    }
    None
}

/// `0x00553f50`: walks a singly linked chain from its head link and returns
/// the *predecessor link* of the first node whose key matches, or the last
/// node when none match.  Result `None` is the head link itself, `Some(i)` is
/// node `i`.
pub fn w19a01_chain_pred(n_nodes: usize, matches: impl Fn(usize) -> bool) -> Option<usize> {
    for i in 0..n_nodes {
        if matches(i) {
            return i.checked_sub(1);
        }
    }
    n_nodes.checked_sub(1)
}

/// `0x00535af0`: swaps the two 32-bit words of a shared-pointer pair
/// (object, control block) with the destination, returning the old
/// destination pair.  The refcount release of the temporary is left to the
/// caller (control-block decrements: use then weak).
pub fn w19a01_swap_pair(dst: &mut [u32; 2], src: &mut [u32; 2]) {
    std::mem::swap(dst, src);
}

/// Handler chosen by `0x00788140` at init (event 0) for a dialog whose
/// kind field (`+0x3a`) is `kind`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W19a01Handler {
    /// `FUN_00788340`, used for kinds 9..=12.
    Variant788340,
    /// `FUN_00787360`, every other kind.
    Variant787360,
}

/// `0x00788140` (init branch): picks the per-dialog handler.
pub fn w19a01_dialog_handler(kind: i16) -> W19a01Handler {
    if !(9..=0xc).contains(&kind) {
        W19a01Handler::Variant787360
    } else {
        W19a01Handler::Variant788340
    }
}

/// Action of the dialog-proc "event 2" (activate) branch of `0x00788140`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum W19a01Activate {
    /// Kinds 2..=6: call `FUN_0074d860(kind - 2, 0)`, handled.
    Select(u16),
    /// Kind 0xf with mode byte 7: `func_0x74_d290(0, 0)`, handled.
    ModeSeven,
    /// Kind 0xf with mode byte 8: `func_0x74_d2a0(0, 0)`, handled.
    ModeEight,
    /// Kind 0x11: calls the non-returning `FUN_005b4cd0`.
    Terminal,
    /// Nothing to do; handled (returns 1).
    None,
}

/// `0x00788140` (event 2): decides the activate action.  `campaign_flag`
/// is the global tested against zero (non-zero disables everything);
/// `mode` is the byte at the mode global.
pub fn w19a01_dialog_activate(kind: u16, campaign_flag: u32, mode: u8) -> W19a01Activate {
    if campaign_flag != 0 {
        return W19a01Activate::None;
    }
    match kind {
        2..=6 => W19a01Activate::Select(kind - 2),
        0xf if mode == 7 => W19a01Activate::ModeSeven,
        0xf if mode == 8 => W19a01Activate::ModeEight,
        0x11 => W19a01Activate::Terminal,
        _ => W19a01Activate::None,
    }
}

#[cfg(test)]
mod tests_wave19_agent01 {
    use super::*;

    #[test]
    fn read_record_retries() {
        let mut calls = vec![];
        let mut chunks = vec![100usize, 24].into_iter();
        let r = w19a01_read_record(|o, w| {
            calls.push((o, w));
            chunks.next().unwrap_or(0)
        });
        assert_eq!(r, Ok(124));
        assert_eq!(calls, vec![(0, 0x7c), (100, 24)]);
        assert_eq!(w19a01_read_record(|_, _| 0), Err("Premature end of stream"));
        assert_eq!(
            w19a01_read_record(|_, w| if w == 0x7c { 10 } else { 0 }),
            Err("Premature end of stream")
        );
    }

    #[test]
    fn hash_and_chain() {
        let buckets = [None, Some(0), None, Some(2)];
        let next = |n: usize| if n == 0 { Some(1) } else { None };
        assert_eq!(w19a01_hash_find(4, 5, &buckets, next, |n| n == 1), Some(1));
        assert_eq!(w19a01_hash_find(4, 5, &buckets, next, |_| false), None);
        assert_eq!(w19a01_hash_find(4, 2, &buckets, next, |_| true), None);
        assert_eq!(w19a01_chain_pred(0, |_| true), None);
        assert_eq!(w19a01_chain_pred(3, |i| i == 0), None);
        assert_eq!(w19a01_chain_pred(3, |i| i == 2), Some(1));
        assert_eq!(w19a01_chain_pred(3, |_| false), Some(2));
    }

    #[test]
    fn swap_and_dialog() {
        let (mut a, mut b) = ([1, 2], [3, 4]);
        w19a01_swap_pair(&mut a, &mut b);
        assert_eq!((a, b), ([3, 4], [1, 2]));
        assert_eq!(w19a01_dialog_handler(8), W19a01Handler::Variant787360);
        assert_eq!(w19a01_dialog_handler(9), W19a01Handler::Variant788340);
        assert_eq!(w19a01_dialog_handler(12), W19a01Handler::Variant788340);
        assert_eq!(w19a01_dialog_handler(13), W19a01Handler::Variant787360);
        assert_eq!(w19a01_dialog_activate(4, 0, 0), W19a01Activate::Select(2));
        assert_eq!(w19a01_dialog_activate(4, 1, 0), W19a01Activate::None);
        assert_eq!(w19a01_dialog_activate(0xf, 0, 7), W19a01Activate::ModeSeven);
        assert_eq!(w19a01_dialog_activate(0xf, 0, 8), W19a01Activate::ModeEight);
        assert_eq!(w19a01_dialog_activate(0xf, 0, 1), W19a01Activate::None);
        assert_eq!(w19a01_dialog_activate(0x11, 0, 0), W19a01Activate::Terminal);
    }
}
