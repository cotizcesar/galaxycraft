#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave33/agent53.rs
// ===========================================================================
/// `0x00caaabf`: frees a null-terminated pointer table (CRT env/argv style).
/// Every entry up to the first zero is released through `free`, then the
/// table itself (passed as `table_handle`) is released last.  A `None`
/// table does nothing.  Returns the number of calls made to `free`.
pub fn w33a53_free_ptr_table(
    table: Option<&[u32]>,
    table_handle: u32,
    free: &mut dyn FnMut(u32),
) -> usize {
    let Some(t) = table else { return 0 };
    let mut n = 0;
    for &p in t.iter().take_while(|&&p| p != 0) {
        free(p);
        n += 1;
    }
    free(table_handle);
    n + 1
}

#[cfg(test)]
mod tests_wave33_agent53 {
    use super::*;

    #[test]
    fn frees_entries_then_table() {
        let mut log = Vec::new();
        let n = w33a53_free_ptr_table(Some(&[5, 6, 0, 9]), 100, &mut |p| log.push(p));
        assert_eq!(log, vec![5, 6, 100]);
        assert_eq!(n, 3);
    }

    #[test]
    fn none_is_noop() {
        let mut c = 0;
        assert_eq!(w33a53_free_ptr_table(None, 1, &mut |_| c += 1), 0);
        assert_eq!(c, 0);
    }
}
