#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent01.rs
// ===========================================================================
// Wave 32 translation batch 01.
//
// Skipped: FUN_00554ec0 (non-returning call / spin loop),
// FUN_005617e0 (forwards to an unmodelled callee).

/// `0x00550360`: builds the lower-cased locale asset path
/// `locales/<locale>/assets/<name>` (ASCII lower-casing per byte).
pub fn w32a01_locale_asset_path(name: &str, locale: &str) -> String {
    format!("locales/{}/assets/{}", locale, name).to_ascii_lowercase()
}

#[cfg(test)]
mod tests_wave32_agent01 {
    use super::*;

    #[test]
    fn path_is_lowercased() {
        assert_eq!(
            w32a01_locale_asset_path("Foo.DDS", "enUS"),
            "locales/enus/assets/foo.dds"
        );
    }
}
