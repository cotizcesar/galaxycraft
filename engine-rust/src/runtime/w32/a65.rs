#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent65.rs
// ===========================================================================
// Wave 32 agent 65 translations (runtime helpers).

/// `0x0093fe50`: returns 1 when `path` is non-empty and the attribute query
/// yields a value that is not `0xffffffff` and lacks the directory bit
/// (`0x10`); otherwise 0.  `attrs` models the callee at `FUN_0093ff30`.
pub fn w32a65_is_regular_file(path: Option<&str>, attrs: impl FnOnce(&str) -> u32) -> u32 {
    match path {
        Some(p) if !p.is_empty() => {
            let a = attrs(p);
            u32::from(a != 0xffff_ffff && (a & 0x10) == 0)
        }
        _ => 0,
    }
}

/// Allocator vtable model used by the two helpers below.
pub trait W32a65Allocator {
    /// vtable slot +4: allocate(size, align)
    fn alloc(&mut self, size: u32, align: u32) -> u32;
    /// vtable slot +8: realloc(ptr, size, align)
    fn realloc(&mut self, ptr: u32, size: u32, align: u32) -> u32;
    /// vtable slot +0xc: usable size(ptr, size)
    fn size_of(&mut self, ptr: u32, size: u32) -> u32;
    /// vtable slot +0x10: free(ptr)
    fn free(&mut self, ptr: u32);
}

/// `0x0094be90`: realloc-style entry.  A null `ptr` allocates
/// `max(size,1)` bytes aligned 8; a zero `size` frees and returns 0;
/// otherwise reallocates to `size + 0x10` aligned 8.
pub fn w32a65_realloc(a: &mut dyn W32a65Allocator, ptr: u32, size: u32) -> u32 {
    if ptr == 0 {
        let n = if size != 0 { size } else { 1 };
        return a.alloc(n, 8);
    }
    if size == 0 {
        a.free(ptr);
        return 0;
    }
    a.realloc(ptr, size.wrapping_add(0x10), 8)
}

/// `0x0094bf00`: returns 0 for a null `ptr`, else the allocator's size
/// query for `(ptr, size + 0x10)`.
pub fn w32a65_msize(a: &mut dyn W32a65Allocator, ptr: u32, size: u32) -> u32 {
    if ptr == 0 {
        return 0;
    }
    a.size_of(ptr, size.wrapping_add(0x10))
}

#[cfg(test)]
mod tests_wave32_agent65 {
    use super::*;

    #[derive(Default)]
    struct M(Vec<String>);
    impl W32a65Allocator for M {
        fn alloc(&mut self, s: u32, a: u32) -> u32 {
            self.0.push(format!("a{s},{a}"));
            100
        }
        fn realloc(&mut self, p: u32, s: u32, a: u32) -> u32 {
            self.0.push(format!("r{p},{s},{a}"));
            200
        }
        fn size_of(&mut self, p: u32, s: u32) -> u32 {
            self.0.push(format!("s{p},{s}"));
            s
        }
        fn free(&mut self, p: u32) {
            self.0.push(format!("f{p}"));
        }
    }

    #[test]
    fn file_check() {
        assert_eq!(w32a65_is_regular_file(None, |_| 0), 0);
        assert_eq!(w32a65_is_regular_file(Some(""), |_| 0), 0);
        assert_eq!(w32a65_is_regular_file(Some("x"), |_| 0xffff_ffff), 0);
        assert_eq!(w32a65_is_regular_file(Some("x"), |_| 0x10), 0);
        assert_eq!(w32a65_is_regular_file(Some("x"), |_| 0x20), 1);
    }

    #[test]
    fn realloc_paths() {
        let mut m = M::default();
        assert_eq!(w32a65_realloc(&mut m, 0, 0), 100);
        assert_eq!(w32a65_realloc(&mut m, 0, 5), 100);
        assert_eq!(w32a65_realloc(&mut m, 7, 0), 0);
        assert_eq!(w32a65_realloc(&mut m, 7, 5), 200);
        assert_eq!(m.0, ["a1,8", "a5,8", "f7", "r7,21,8"]);
    }

    #[test]
    fn msize_paths() {
        let mut m = M::default();
        assert_eq!(w32a65_msize(&mut m, 0, 4), 0);
        assert_eq!(w32a65_msize(&mut m, 3, 4), 20);
    }
}
