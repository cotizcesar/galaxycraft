#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent89.rs
// ===========================================================================
// Wave 32 translation batch 89: RTTI-style type-name compatibility checks.

/// Type-name comparison shared by the three routines: when the two names are
/// the same pointer or the bounded compare (`FUN_00ca0b90`, limit
/// INT_MAX) is zero, they match.
pub fn w32a89_names_match(a: &str, b: &str) -> bool {
    a == b
}

/// Resolves a type name: the virtual slot `+0x18` result when present,
/// otherwise the fallback name (`FUN_00a7b8c0`).
pub fn w32a89_resolve_name<'a>(slot_name: Option<&'a str>, fallback: &'a str) -> &'a str {
    slot_name.unwrap_or(fallback)
}

/// `0x00a7d800`: the other object (virtual slot 0 of `param_1`) must exist,
/// its type name must match ours, and the parallel chains of base names
/// (`+0x38`, next `+0x20`) must match over their common length.
pub fn w32a89_chain_compatible(
    other_present: bool,
    own_name: &str,
    other_name: &str,
    own_chain: &[&str],
    other_chain: &[&str],
) -> bool {
    if !other_present || !w32a89_names_match(own_name, other_name) {
        return false;
    }
    own_chain
        .iter()
        .zip(other_chain.iter())
        .all(|(a, b)| w32a89_names_match(a, b))
}

/// `0x00a7d8d0`: object from virtual slot `+8` must exist and share our type
/// name.
pub fn w32a89_slot8_compatible(other: Option<&str>, own_name: &str) -> bool {
    other.is_some_and(|o| w32a89_names_match(o, own_name))
}

/// `0x00a7d940`: object from virtual slot `+0x28` must exist and share our
/// type name.
pub fn w32a89_slot28_compatible(other: Option<&str>, own_name: &str) -> bool {
    other.is_some_and(|o| w32a89_names_match(o, own_name))
}

#[cfg(test)]
mod tests_wave32_agent89 {
    use super::*;

    #[test]
    fn chain() {
        assert!(w32a89_chain_compatible(true, "A", "A", &["x", "y"], &["x"]));
        assert!(!w32a89_chain_compatible(true, "A", "A", &["x"], &["z"]));
        assert!(!w32a89_chain_compatible(false, "A", "A", &[], &[]));
        assert!(!w32a89_chain_compatible(true, "A", "B", &[], &[]));
    }

    #[test]
    fn slots() {
        assert!(w32a89_slot8_compatible(Some("A"), "A"));
        assert!(!w32a89_slot8_compatible(None, "A"));
        assert!(!w32a89_slot28_compatible(Some("B"), "A"));
        assert_eq!(w32a89_resolve_name(None, "f"), "f");
    }
}
