#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave21/agent16.rs
// ===========================================================================
// Wave 21 translation batch 16.
//
// Window: queue: 20 addresses, module `runtime.rs`.

/// Inline-capable empty string record written by the static-init stubs
/// (id at +0, data pointer at +4, length at +8, capacity flags at +0xc,
/// first buffer byte at +0x10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub struct w21a16_StaticString {
    pub id: u32,
    pub data_ptr: u32,
    pub len: u32,
    pub cap: u32,
    pub first_byte: u8,
}

/// Shared body of the three static string initialisers: sets `id`, points
/// the data pointer at the inline buffer, clears length and first byte and
/// stores capacity `0x8000000f`.
pub fn w21a16_init_static_string(id: u32, inline_buf: u32) -> w21a16_StaticString {
    w21a16_StaticString {
        id,
        data_ptr: inline_buf,
        len: 0,
        cap: 0x8000_000f,
        first_byte: 0,
    }
}

/// `0x005d4630`: forwards the dword at `this + 0xc` to the callee.
pub fn w21a16_forward_field_0c<R>(this_words: &[u32], callee: impl FnOnce(u32) -> R) -> R {
    callee(this_words[3])
}

/// `0x005d48c6`: looks up `table[index & 0xffff]`, passes it to the first
/// callee, then runs the second callee.
pub fn w21a16_table_lookup_then<R>(
    table: &[u32],
    index: u32,
    first: impl FnOnce(u32),
    second: impl FnOnce() -> R,
) -> R {
    first(table[(index & 0xffff) as usize]);
    second()
}

/// `0x005e221a`: static string init with id `0x1f` at buffer `0x00f042e8`.
pub fn w21a16_init_string_1f() -> w21a16_StaticString {
    w21a16_init_static_string(0x1f, 0x00f0_42e8)
}

/// `0x005e27a6`: static string init with id `0x2c` at buffer `0x00f04488`.
pub fn w21a16_init_string_2c() -> w21a16_StaticString {
    w21a16_init_static_string(0x2c, 0x00f0_4488)
}

/// `0x005e2889`: static string init with id `0x2e` at buffer `0x00f044c8`.
pub fn w21a16_init_string_2e() -> w21a16_StaticString {
    w21a16_init_static_string(0x2e, 0x00f0_44c8)
}

#[cfg(test)]
mod tests_wave21_agent16 {
    use super::*;

    #[test]
    fn static_strings() {
        let a = w21a16_init_string_1f();
        assert_eq!(
            (a.id, a.data_ptr, a.len, a.cap),
            (0x1f, 0xf042e8, 0, 0x8000000f)
        );
        assert_eq!(w21a16_init_string_2c().id, 0x2c);
        assert_eq!(w21a16_init_string_2c().data_ptr, 0xf04488);
        assert_eq!(w21a16_init_string_2e().id, 0x2e);
        assert_eq!(w21a16_init_string_2e().data_ptr, 0xf044c8);
        assert_eq!(a.first_byte, 0);
    }

    #[test]
    fn forward_and_lookup() {
        assert_eq!(w21a16_forward_field_0c(&[1, 2, 3, 9], |v| v + 1), 10);
        let mut seen = 0;
        let r = w21a16_table_lookup_then(&[5, 6, 7], 0x10001, |v| seen = v, || 3);
        assert_eq!((seen, r), (6, 3));
    }
}
