#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent67.rs
// ===========================================================================
// Wave 33 translation batch 67.
//
// Skipped: FUN_00cb57cc (locale monetary setup; depends on unmodelled
// locale-info callee and heap primitives).

/// `0x00cb5481`: CRT `_copy_environ`.  Duplicates a NULL-terminated array of
/// C strings; each entry is copied including its terminator.  `None` input
/// yields `None` (the original returns 0); allocation failure paths end in
/// non-returning calls and are not modelled.
pub fn w33a67_copy_environ(src: Option<&[Vec<u8>]>) -> Option<Vec<Vec<u8>>> {
    src.map(|rows| rows.to_vec())
}

/// `0x00cb56ce`: CRT `__free_lconv_mon`.  `fields` are the lconv string
/// pointers (indexed by byte offset / 4) and `defaults` the C-locale
/// pointers at the same offsets.  Returns the offsets (in bytes) whose value
/// differs from the default and therefore get freed, in original order
/// (0xc..=0x24, then 0x38..=0x4c).  A null record frees nothing.
pub fn w33a67_free_lconv_mon_offsets(fields: Option<(&[usize], &[usize])>) -> Vec<usize> {
    let Some((fields, defaults)) = fields else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for off in (0x0cusize..=0x24)
        .step_by(4)
        .chain((0x38..=0x4c).step_by(4))
    {
        let i = off / 4;
        if fields.get(i) != defaults.get(i) {
            out.push(off);
        }
    }
    out
}

#[cfg(test)]
mod tests_wave33_agent67 {
    use super::*;

    #[test]
    fn copy_environ() {
        assert_eq!(w33a67_copy_environ(None), None);
        let v = vec![b"A=1\0".to_vec()];
        assert_eq!(w33a67_copy_environ(Some(&v)), Some(v.clone()));
    }

    #[test]
    fn free_lconv() {
        assert!(w33a67_free_lconv_mon_offsets(None).is_empty());
        let d = vec![0usize; 20];
        let mut f = d.clone();
        assert!(w33a67_free_lconv_mon_offsets(Some((&f, &d))).is_empty());
        f[3] = 5;
        f[0x4c / 4] = 9;
        f[0x28 / 4] = 7; // not freed
        assert_eq!(
            w33a67_free_lconv_mon_offsets(Some((&f, &d))),
            vec![0xc, 0x4c]
        );
    }
}
