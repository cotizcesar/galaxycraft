#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave25/agent12.rs
// ===========================================================================
// Wave 25 agent 12: runtime helpers.

/// Four-dword record built by `0x007e2290`: family, converted port,
/// address dword, and a zeroed trailing dword.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct W25a12Sockaddr {
    pub family: u16,
    pub port: u16,
    pub addr: u32,
    pub tail: u32,
}

/// `0x007e2290`: copies a 16-byte address record from `src`
/// (`[family:u16, port:u16, addr:u32]`), passing the port through the
/// byte-order conversion import `convert` (imported thunk, unresolved).
pub fn w25a12_copy_sockaddr(src: &[u8; 8], convert: impl Fn(u16) -> u16) -> W25a12Sockaddr {
    let family = u16::from_le_bytes([src[0], src[1]]);
    let port = convert(u16::from_le_bytes([src[2], src[3]]));
    let addr = u32::from_le_bytes([src[4], src[5], src[6], src[7]]);
    W25a12Sockaddr {
        family,
        port,
        addr,
        tail: 0,
    }
}

#[cfg(test)]
mod tests_wave25_agent12 {
    use super::*;

    #[test]
    fn copy_sockaddr_converts_port_only() {
        let src = [2, 0, 0x12, 0x34, 1, 2, 3, 4];
        let r = w25a12_copy_sockaddr(&src, u16::swap_bytes);
        assert_eq!(r.family, 2);
        assert_eq!(r.port, 0x1234);
        assert_eq!(r.addr, 0x0403_0201);
        assert_eq!(r.tail, 0);
    }
}
