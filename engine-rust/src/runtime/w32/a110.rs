#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent110.rs
// ===========================================================================
// Wave 32 agent 110: GPU vendor id formatting.

/// Vendor name for a PCI vendor id (callee of the formatter; not cited).
pub fn w32a110_vendor_name(id: u16) -> Option<&'static str> {
    match id {
        0x1414 => Some("Microsoft"),
        0x1002 => Some("AMD"),
        0x106b => Some("Apple"),
        0x10de => Some("NVidia"),
        0x4351 => Some("Qualcomm"),
        0x8086 => Some("Intel"),
        _ => None,
    }
}

/// `0x00c12a60`: formats a vendor id as its known name, or as `0x%04x` of
/// the low 16 bits when the vendor is unknown.
pub fn w32a110_format_vendor(id: u32) -> String {
    match w32a110_vendor_name(id as u16) {
        Some(n) => n.to_string(),
        None => format!("0x{:04x}", id & 0xffff),
    }
}

#[cfg(test)]
mod tests_wave32_agent110 {
    use super::*;

    #[test]
    fn known_and_unknown() {
        assert_eq!(w32a110_format_vendor(0x10de), "NVidia");
        assert_eq!(w32a110_format_vendor(0x8086), "Intel");
        assert_eq!(w32a110_format_vendor(0x1_abcd), "0xabcd");
        assert_eq!(w32a110_format_vendor(0x12), "0x0012");
    }
}
