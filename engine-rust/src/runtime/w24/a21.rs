#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent21.rs
// ===========================================================================
// Wave 24 agent 21: tick-rate helpers, time formatting, sorted span insert,
// palette tables.  Skipped: FUN_0075be20, FUN_0075bf70, FUN_0075c170,
// FUN_0075c640, FUN_0075db30, FUN_00760a30, FUN_00760ac0, FUN_007621d0,
// FUN_007621f0, FUN_00762210, FUN_00762834 (non-returning calls, SEH glue,
// allocator wrappers).

/// `0x0075c0e0`: returns the game-speed table entry (`DAT_00d508e0`, u32
/// per speed) selected by `alt_index` when `use_alt` is true, else `index`.
pub fn w24a21_speed_entry(use_alt: bool, index: u8, alt_index: u8, table: &[u32]) -> u32 {
    let i = if use_alt { alt_index } else { index };
    table[i as usize]
}

/// `0x0075c1e0`: returns `Some(target)` when the resync call should be made
/// with `target = min(wanted, cap)`.  `cur` is the u32 at +0x14c, `min_cur`
/// is the lower bound; `speed_mult` is the speed table entry.
pub fn w24a21_resync_target(
    enabled: bool,
    ready: bool,
    cur: u32,
    min_cur: u32,
    cap: u32,
    wanted: u32,
    speed_mult: u32,
) -> Option<u32> {
    if !enabled || !ready || min_cur > cur {
        return None;
    }
    let target = wanted.min(cap);
    let diff = target.wrapping_sub(cur) as i32;
    let abs = diff.wrapping_abs() as u32;
    if abs.wrapping_mul(speed_mult) / 1000 > 1 {
        Some(target)
    } else {
        None
    }
}

/// `0x0075c540`: formats seconds as `H:MM:SS` when at least an hour, else
/// `MM:SS` (minutes taken modulo 60).
pub fn w24a21_format_time(seconds: u32) -> String {
    let h = seconds / 0xe10;
    let m = (seconds / 0x3c) % 0x3c;
    let s = seconds % 0x3c;
    if h != 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

/// Result of `0x0075c5b0`.
#[derive(Debug, PartialEq, Eq)]
pub enum W24a21TickResult {
    /// Second counter changed and the callback result was returned.
    Changed(i32),
    /// Unchanged: `product * 0x10624dd3` (wrapping).
    Unchanged(u32),
    /// Branch where the flag at `DAT_011a456c` is non-zero (non-returning).
    Flagged,
}

/// `0x0075c5b0`: `product = speed_mult * frame_counter`; tracks
/// `product / 1000` in `last_seconds`.
pub fn w24a21_tick_seconds(
    speed_mult: u32,
    frame_counter: u32,
    last_seconds: &mut u32,
    flagged: bool,
    on_change: impl FnOnce() -> i32,
) -> W24a21TickResult {
    let product = speed_mult.wrapping_mul(frame_counter);
    let secs = product / 1000;
    let changed = secs != *last_seconds;
    if changed {
        *last_seconds = secs;
    }
    if flagged {
        return W24a21TickResult::Flagged;
    }
    if changed {
        W24a21TickResult::Changed(on_change())
    } else {
        W24a21TickResult::Unchanged(product.wrapping_mul(0x10624dd3))
    }
}

/// `0x0075cf70`: dialog helper.  `find(kind)` models the control lookup
/// (`2`, later `-2`), `selected()` the selection byte (`-1` = none),
/// `item_for(sel)` the item value, `apply(ctrl, item)` the setter, and
/// `clear_flag(ctrl)` writes 0 to byte +0x51.  `release(ctrl)` is the final
/// call on the second lookup.
pub fn w24a21_dialog_apply_selection<C: Copy>(
    mut find: impl FnMut(i32) -> Option<C>,
    selected: impl FnOnce() -> i8,
    item_for: impl FnOnce(i8) -> u32,
    apply: impl FnOnce(C, u32),
    clear_flag: impl FnOnce(C),
    release: impl FnOnce(C),
) {
    let Some(ctrl) = find(2) else { return };
    let sel = selected();
    if sel == -1 {
        return;
    }
    apply(ctrl, item_for(sel));
    clear_flag(ctrl);
    if let Some(other) = find(-2) {
        release(other);
    }
}

/// `0x0075da40`: reads the u16 at `idx * 0x40 + 0x449fe` of `buf` and
/// returns it combined with the high half of the base pointer.
pub fn w24a21_read_tile_word(buf: &[u8], base_high: u16, idx: u16) -> Option<u32> {
    let o = idx as usize * 0x40 + 0x449fe;
    let b = buf.get(o..o + 2)?;
    Some(((base_high as u32) << 16) | u16::from_le_bytes([b[0], b[1]]) as u32)
}

fn w24a21_b7(rec: &[i16; 4]) -> u8 {
    (rec[3] >> 8) as u8
}

/// `0x0075ff00`: sorted insert / merge of 8-byte span records
/// `[key, lo, hi, flags<<8|..]` into `buf[*cursor..*end]`.  `ext` is the i16
/// table at context +0x20 indexed by the 2-bit fields of byte 7.  `buf`
/// must have room for one more record.
pub fn w24a21_insert_span(
    rec: &[i16; 4],
    buf: &mut [[i16; 4]],
    cursor: &mut usize,
    end: &mut usize,
    ext: &[i16; 4],
) {
    let e = *end;
    let mut p = *cursor;
    let rb = w24a21_b7(rec);
    while p < e {
        let cur = buf[p];
        let cb = w24a21_b7(&cur);
        let s = cur[0];
        if rec[0] <= s
            && (s != rec[0]
                || ext[(rb & 3) as usize] as i32 + rec[1] as i32
                    <= ext[((cb >> 2) & 3) as usize] as i32 + cur[2] as i32)
        {
            if s == rec[0]
                && ext[(rb & 3) as usize] as i32 + rec[1] as i32
                    <= cur[2] as i32 + ext[((cb >> 2) & 3) as usize] as i32 + 1
                && cur[1] as i32 + ext[(cb & 3) as usize] as i32 - 1
                    <= ext[((rb >> 2) & 3) as usize] as i32 + rec[2] as i32
            {
                if rec[1] < buf[p][1] {
                    buf[p][1] = rec[1];
                }
                if buf[p][2] < rec[2] {
                    buf[p][2] = rec[2];
                }
                *cursor = p;
                return;
            }
            let mut q = e;
            while p < q {
                buf[q] = buf[q - 1];
                q -= 1;
            }
            break;
        }
        p += 1;
    }
    buf[p] = *rec;
    *cursor = p;
    *end = e + 1;
}

/// `0x00760c20` (table build only; allocations omitted): expands palette
/// entries (16 u16 words each) into four u32 columns, then scatters them per
/// map cell (`map[i] & 0x7fff`) into `out` (row stride 0x400 u32, planes at
/// +0x100 each).  `width`/`height` are the u16 fields at +0xe4/+0xe6.
pub fn w24a21_build_palette_table(
    pal_words: &[u16],
    entry_count: usize,
    map: &[u16],
    width: usize,
    height: usize,
    out: &mut [u32],
) {
    let mut table = vec![0u32; entry_count * 4];
    for i in 0..entry_count {
        let w = &pal_words[i * 16..i * 16 + 16];
        for k in 0..4 {
            let v = (((((w[4 * k + 3] as u32) << 8 | (w[4 * k + 2] & 1) as u32) << 8
                | (w[4 * k + 1] & 1) as u32)
                << 8)
                | (w[4 * k] & 1) as u32)
                ^ 0xff01_0101;
            table[i * 4 + k] = v << 7;
        }
    }
    let mut mi = 0;
    for y in 0..height {
        for x in 0..width {
            let idx = (map[mi] & 0x7fff) as usize;
            mi += 1;
            let o = y * 0x400 + x;
            for k in 0..4 {
                out[o + k * 0x100] = table[idx * 4 + k];
            }
        }
    }
}

/// `0x00762590`: offset helper.  With a record `[a,b,c,d]` (i16 at
/// +0x10..+0x16) returns `(a+b+c+d)*8 + 0xb7a60`, else `0xb7a28`.
pub fn w24a21_buffer_offset(rec: Option<[i16; 4]>) -> i32 {
    match rec {
        Some(r) => r.iter().map(|&v| v as i32).sum::<i32>().wrapping_mul(8) + 0xb7a60,
        None => 0xb7a28,
    }
}

#[cfg(test)]
mod tests_wave24_agent21 {
    use super::*;

    #[test]
    fn speed_and_resync() {
        let t = [1, 2, 3];
        assert_eq!(w24a21_speed_entry(true, 0, 2, &t), 3);
        assert_eq!(w24a21_speed_entry(false, 1, 2, &t), 2);
        assert_eq!(
            w24a21_resync_target(true, true, 10, 5, 100, 50, 1000),
            Some(50)
        );
        assert_eq!(w24a21_resync_target(true, true, 10, 5, 100, 11, 1000), None);
        assert_eq!(
            w24a21_resync_target(true, true, 10, 11, 100, 50, 1000),
            None
        );
        assert_eq!(
            w24a21_resync_target(false, true, 10, 5, 100, 50, 1000),
            None
        );
        assert_eq!(
            w24a21_resync_target(true, true, 10, 5, 20, 50, 1000),
            Some(20)
        );
    }

    #[test]
    fn time_format() {
        assert_eq!(w24a21_format_time(59), "00:59");
        assert_eq!(w24a21_format_time(3661), "1:01:01");
        assert_eq!(w24a21_format_time(125), "02:05");
    }

    #[test]
    fn tick() {
        let mut last = 0;
        assert_eq!(
            w24a21_tick_seconds(1000, 2, &mut last, false, || 7),
            W24a21TickResult::Changed(7)
        );
        assert_eq!(last, 2);
        assert_eq!(
            w24a21_tick_seconds(1000, 2, &mut last, false, || 7),
            W24a21TickResult::Unchanged(2000u32.wrapping_mul(0x10624dd3))
        );
        assert_eq!(
            w24a21_tick_seconds(1000, 3, &mut last, true, || 7),
            W24a21TickResult::Flagged
        );
    }

    #[test]
    fn dialog() {
        let log = std::cell::RefCell::new(Vec::new());
        w24a21_dialog_apply_selection(
            Some,
            || 3,
            |s| s as u32 * 10,
            |c, v| log.borrow_mut().push((c, v)),
            |_| log.borrow_mut().push((99, 0)),
            |c| log.borrow_mut().push((c, 1)),
        );
        assert_eq!(*log.borrow(), vec![(2, 30), (99, 0), (-2, 1)]);
        let mut n = 0;
        w24a21_dialog_apply_selection(Some, || -1, |_| 0, |_, _| n += 1, |_| {}, |_| {});
        assert_eq!(n, 0);
    }

    #[test]
    fn tile_word() {
        let mut b = vec![0u8; 0x449fe + 0x80];
        b[0x449fe + 0x40] = 0x34;
        b[0x449fe + 0x41] = 0x12;
        assert_eq!(w24a21_read_tile_word(&b, 0xabcd, 1), Some(0xabcd_1234));
        assert_eq!(w24a21_read_tile_word(&b, 0, 100), None);
    }

    #[test]
    fn span_insert() {
        let ext = [0i16; 4];
        let mut buf = [[0i16; 4]; 4];
        buf[0] = [5, 10, 20, 0];
        let mut cur = 0;
        let mut end = 1;
        // smaller key goes first, existing shifts
        w24a21_insert_span(&[3, 0, 1, 0], &mut buf, &mut cur, &mut end, &ext);
        assert_eq!((cur, end), (0, 2));
        assert_eq!(buf[1][0], 5);
        // merge with the key-5 record (adjacent range)
        let mut cur = 1;
        w24a21_insert_span(&[5, 20, 30, 0], &mut buf, &mut cur, &mut end, &ext);
        assert_eq!((cur, end), (1, 2));
        assert_eq!(buf[1], [5, 10, 30, 0]);
        // larger key appended
        let mut cur = 0;
        w24a21_insert_span(&[9, 0, 0, 0], &mut buf, &mut cur, &mut end, &ext);
        assert_eq!((cur, end), (2, 3));
    }

    #[test]
    fn palette() {
        let mut pal = vec![0u16; 16];
        pal[3] = 2;
        let map = [0u16, 0x8000];
        let mut out = vec![0u32; 0x400];
        w24a21_build_palette_table(&pal, 1, &map, 2, 1, &mut out);
        let v = ((2u32 << 24) ^ 0xff01_0101) << 7;
        assert_eq!(out[0], v);
        assert_eq!(out[1], v);
        assert_eq!(
            out[0x100],
            (0x0100_0000 ^ 0xff01_0101 ^ 0x0100_0000) << 7 | 0
        );
    }

    #[test]
    fn offset() {
        assert_eq!(w24a21_buffer_offset(None), 0xb7a28);
        assert_eq!(w24a21_buffer_offset(Some([1, 2, 3, 4])), 80 + 0xb7a60);
    }
}
