#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent124.rs
// ===========================================================================
// Wave 32 translation batch 124.
//
// Skipped: FUN_00c815e0 and FUN_00c81730 (lazy resolver dispatch through
// unmodelled global function pointers).

fn w32a124_lower(c: u16) -> u16 {
    if (0x41..0x5b).contains(&c) {
        c + 0x20
    } else {
        c
    }
}

fn w32a124_at(s: &[u16], i: usize) -> u16 {
    s.get(i).copied().unwrap_or(0)
}

fn w32a124_len(s: &[u16]) -> usize {
    s.iter().position(|&c| c == 0).unwrap_or(s.len())
}

/// Index of the extension dot (or end of string): scans back from the end
/// to a `\` or `.` and returns the dot if found, else the string end.
fn w32a124_ext_pos(s: &[u16]) -> usize {
    let end = w32a124_len(s);
    let mut p = end;
    while p > 0 && w32a124_at(s, p) != 0x5c && w32a124_at(s, p) != 0x2e {
        p -= 1;
    }
    if w32a124_at(s, p) == 0x2e { p } else { end }
}

fn w32a124_cmp_ci(a: &[u16], b: &[u16], limit: usize) -> i32 {
    for i in 0..limit {
        let x = w32a124_lower(w32a124_at(a, i));
        let y = w32a124_lower(w32a124_at(b, i));
        if x != y || x == 0 {
            return x as i32 - y as i32;
        }
    }
    0
}

/// `0x00c82850`: true when `name` (path stripped to its last component)
/// matches `module` case-insensitively, either exactly or by stem when each
/// extension is empty or `.dll`.
pub fn w32a124_module_name_matches(name: &[u16], module: &[u16]) -> bool {
    let start = (0..w32a124_len(name))
        .rev()
        .find(|&i| name[i] == 0x5c)
        .map_or(0, |i| i + 1);
    let name = &name[start..];
    if w32a124_cmp_ci(name, module, usize::MAX) == 0 {
        return true;
    }
    let ea = w32a124_ext_pos(name);
    let eb = w32a124_ext_pos(module);
    if ea != eb {
        return false;
    }
    let dll: Vec<u16> = ".dll".encode_utf16().collect();
    for (s, e) in [(module, eb), (name, ea)] {
        let ext = &s[e.min(s.len())..];
        if w32a124_at(ext, 0) != 0 && w32a124_cmp_ci(ext, &dll, usize::MAX) != 0 {
            return false;
        }
    }
    w32a124_cmp_ci(name, module, ea) == 0
}

#[cfg(test)]
mod tests_wave32_agent124 {
    use super::*;
    fn w(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }
    #[test]
    fn matches() {
        assert!(w32a124_module_name_matches(
            &w("C:\\x\\Foo.dll"),
            &w("foo.dll")
        ));
        assert!(w32a124_module_name_matches(&w("C:\\x\\Foo.dll"), &w("foo")));
        assert!(!w32a124_module_name_matches(&w("foo.exe"), &w("foo.dll")));
        assert!(!w32a124_module_name_matches(&w("foo.dll"), &w("bar.dll")));
    }
}
