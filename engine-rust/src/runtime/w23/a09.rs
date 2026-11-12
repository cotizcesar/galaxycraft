#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave23/agent09.rs
// ===========================================================================
// Wave 23 translation batch 09: popup/error-dialog helpers (pure reference).

/// `0x006f1380`: counts entries of a 4-`u16` stride table whose first word is
/// non-zero and whose second word is `<= limit`; the table ends when the
/// next entry's second word is 0.  Returns `(count, error)` where `error`
/// is true when more than one entry matched (the original then raises).
pub fn w23a09_count_matching(table: &[u16], limit: u32) -> (u32, bool) {
    let mut count = 0u32;
    let mut i = 0usize;
    let mut second = match table.get(1) {
        Some(&v) => v,
        None => return (0, false),
    };
    if second == 0 {
        return (0, false);
    }
    loop {
        if u32::from(second) <= limit && table.get(i).copied().unwrap_or(0) != 0 {
            count += 1;
        }
        second = table.get(i + 5).copied().unwrap_or(0);
        i += 4;
        if second == 0 {
            break;
        }
    }
    (count, count > 1)
}

/// `0x006f0ef0`: clamps a message length to the 0xfe-byte buffer copy size
/// (strlen of the C string, at most 0xfe).
pub fn w23a09_clamped_len(msg: &[u8]) -> usize {
    let n = msg.iter().position(|&b| b == 0).unwrap_or(msg.len());
    n.min(0xfe)
}

/// `0x006f2030`: copies the NUL-terminated name into the 0x100-byte message
/// buffer and builds the popup path `<base>\pOPopup.pcx`; error code is 0xe,
/// kind 8.  Returns `(name_copy, popup_path)`.
pub fn w23a09_popup_o_paths(name: &[u8], base_dir: &[u8]) -> (Vec<u8>, Vec<u8>) {
    w23a09_popup_paths(name, base_dir, b"\\pOPopup.pcx")
}

/// `0x006f2330`: same as the O-popup builder but with the suffix
/// `\pDPopup.pcx` (little-endian dwords); code 0x14,
/// kind 0xc.
pub fn w23a09_popup_d_paths(name: &[u8], base_dir: &[u8]) -> (Vec<u8>, Vec<u8>) {
    w23a09_popup_paths(name, base_dir, b"\\pDPopup.pcx")
}

fn w23a09_popup_paths(name: &[u8], base_dir: &[u8], suffix: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let n = name.iter().position(|&b| b == 0).unwrap_or(name.len());
    let b = base_dir
        .iter()
        .position(|&c| c == 0)
        .unwrap_or(base_dir.len());
    let mut path = base_dir[..b].to_vec();
    path.extend_from_slice(suffix);
    (name[..n].to_vec(), path)
}

/// Outcome of `0x006f1d30`.
#[derive(Debug, PartialEq, Eq)]
pub enum W23a09PopupChoice {
    /// Second string present: both strings are stored, error code 0x13, kind 0x14.
    Dual { first: Vec<u8>, second: Vec<u8> },
    /// Otherwise delegates to the single-name popup (the 0x006f2030 body).
    Single,
}

/// `0x006f1d30`: if `second` is non-empty both strings are latched and the
/// dual-name popup (code 0x13) is raised, otherwise the single-name path runs.
pub fn w23a09_choose_popup(first: &[u8], second: Option<&[u8]>) -> W23a09PopupChoice {
    let cut = |s: &[u8]| s.iter().position(|&b| b == 0).unwrap_or(s.len());
    match second {
        Some(s) if !s.is_empty() && s[0] != 0 => W23a09PopupChoice::Dual {
            first: first[..cut(first)].to_vec(),
            second: s[..cut(s)].to_vec(),
        },
        _ => W23a09PopupChoice::Single,
    }
}

/// `0x006f3460`: child-control index for the first button handler:
/// `param + 0xf`.
pub fn w23a09_child_index_a(param: u8) -> i32 {
    i32::from(param) + 0xf
}

/// `0x006f34d0`: child-control index for the second button handler:
/// `param + 0xf`.
pub fn w23a09_child_index_b(param: u8) -> i32 {
    i32::from(param) + 0xf
}

/// `0x006f0e40`: dispatch on message kind (`kind`): `< 2` runs `default`,
/// then sets the flag from `state == 0` and returns 1; `0xf` runs `other`
/// and returns 1; anything else returns `default`'s result.
pub fn w23a09_dispatch(
    kind: u16,
    mut default: impl FnMut() -> u32,
    mut other: impl FnMut(),
    state: u32,
    mut set_flag: impl FnMut(bool),
) -> u32 {
    if kind < 2 {
        default();
        set_flag(state == 0);
        1
    } else if kind == 0xf {
        other();
        1
    } else {
        default()
    }
}

/// `0x006f3500` (tail): releases the dialog slot.  `dialog` is the 0x968-byte
/// block: clears bit 0 of the byte at `0x164 + 0x20 * dialog[0x967]`, clears
/// bit 0 of the dword at `0x948`, zeroes `dialog[0x967]`.  Returns the new
/// owner pointer value (`shared` when it is non-zero and even, else 0).
pub fn w23a09_release_slot(dialog: Option<&mut [u8]>, shared: u32) -> u32 {
    if let Some(d) = dialog {
        let idx = 0x164 + usize::from(d[0x967]) * 0x20;
        d[idx] &= 0xfe;
        d[0x948] &= 0xfe;
        d[0x967] = 0;
    }
    if shared & 1 == 0 && shared != 0 {
        shared
    } else {
        0
    }
}

#[cfg(test)]
mod tests_wave23_agent09 {
    use super::*;

    #[test]
    fn count_matching() {
        // entries: [a,1,_,_] [b,2,..] terminator second=0
        let t = [1u16, 1, 0, 0, 1, 2, 0, 0, 0, 0, 0, 0];
        assert_eq!(w23a09_count_matching(&t, 2), (2, true));
        assert_eq!(w23a09_count_matching(&t, 1), (1, false));
        assert_eq!(w23a09_count_matching(&[0, 0], 5), (0, false));
        assert_eq!(w23a09_count_matching(&[], 5), (0, false));
    }

    #[test]
    fn clamp() {
        assert_eq!(w23a09_clamped_len(b"abc\0def"), 3);
        assert_eq!(w23a09_clamped_len(&[b'x'; 400]), 0xfe);
    }

    #[test]
    fn popups() {
        let (n, p) = w23a09_popup_o_paths(b"nm\0zz", b"C:\\SC\0");
        assert_eq!(n, b"nm");
        assert_eq!(p, b"C:\\SC\\pOPopup.pcx");
        let (_, p) = w23a09_popup_d_paths(b"a", b"D");
        assert_eq!(p, b"D\\pDPopup.pcx");
        assert_eq!(&1346662492u32.to_le_bytes(), b"\\pDP");
        assert_eq!(
            w23a09_choose_popup(b"a", Some(b"b")),
            W23a09PopupChoice::Dual {
                first: b"a".to_vec(),
                second: b"b".to_vec()
            }
        );
        assert_eq!(
            w23a09_choose_popup(b"a", Some(b"")),
            W23a09PopupChoice::Single
        );
        assert_eq!(w23a09_choose_popup(b"a", None), W23a09PopupChoice::Single);
    }

    #[test]
    fn child_index() {
        assert_eq!(w23a09_child_index_a(0), 15);
        assert_eq!(w23a09_child_index_b(3), 18);
    }

    #[test]
    fn dispatch() {
        let mut flag = None;
        assert_eq!(w23a09_dispatch(1, || 7, || {}, 0, |b| flag = Some(b)), 1);
        assert_eq!(flag, Some(true));
        assert_eq!(w23a09_dispatch(0xf, || 7, || {}, 1, |_| {}), 1);
        assert_eq!(w23a09_dispatch(5, || 7, || {}, 1, |_| {}), 7);
    }

    #[test]
    fn release() {
        let mut d = vec![0xffu8; 0x968];
        d[0x967] = 2;
        assert_eq!(w23a09_release_slot(Some(&mut d), 0x100), 0x100);
        assert_eq!(d[0x164 + 0x40], 0xfe);
        assert_eq!(d[0x948], 0xfe);
        assert_eq!(d[0x967], 0);
        assert_eq!(w23a09_release_slot(None, 1), 0);
        assert_eq!(w23a09_release_slot(None, 0), 0);
    }
}
