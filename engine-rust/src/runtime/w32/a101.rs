#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent101.rs
// ===========================================================================
/// Result of the virtual query slot at vtable offset 0x34 used by `0x00af1c50`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct W32a101QueryInfo {
    /// First byte of the 0x44-byte local record (bit 1 = size valid).
    pub flags: u8,
    /// Two dwords at record offsets 0x10 and 0x14.
    pub size: [u32; 2],
}

/// `0x00af1c50`: refreshes state (`pre`), queries the object through its
/// virtual slot (`query`, returns status plus record) and, when the status is
/// zero and flag bit 1 is set, returns the two-dword value; flag clear yields
/// status 2.  Non-zero query status is propagated.
pub fn w32a101_query_size(
    pre: impl FnOnce(),
    query: impl FnOnce() -> (i32, W32a101QueryInfo),
) -> Result<[u32; 2], i32> {
    pre();
    let (status, info) = query();
    if status != 0 {
        return Err(status);
    }
    if info.flags & 2 == 0 {
        return Err(2);
    }
    Ok(info.size)
}

/// `0x00b1ccf0`: initialises a record: dwords 0 and 1 cleared, then the
/// sub-object starting at dword 2 is initialised by `init_tail`.
pub fn w32a101_init_pair(dw: &mut [u32], init_tail: impl FnOnce(&mut [u32])) {
    dw[0] = 0;
    dw[1] = 0;
    init_tail(&mut dw[2..]);
}

#[cfg(test)]
mod tests_wave32_agent101 {
    use super::*;

    #[test]
    fn query_size() {
        let ok = W32a101QueryInfo {
            flags: 2,
            size: [7, 9],
        };
        assert_eq!(w32a101_query_size(|| {}, || (0, ok)), Ok([7, 9]));
        let no = W32a101QueryInfo {
            flags: 1,
            size: [7, 9],
        };
        assert_eq!(w32a101_query_size(|| {}, || (0, no)), Err(2));
        assert_eq!(w32a101_query_size(|| {}, || (5, ok)), Err(5));
    }

    #[test]
    fn init_pair() {
        let mut v = [1u32, 2, 3, 4];
        w32a101_init_pair(&mut v, |t| t[0] = 99);
        assert_eq!(v, [0, 0, 99, 4]);
    }
}
