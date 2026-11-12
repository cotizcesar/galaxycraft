#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave32/agent79.rs
// ===========================================================================
// Wave 32 translation batch 79: libogg page sync.
//
// Skipped: FUN_009f38e0, FUN_009f3960 (libvorbis mapping/floor setup that
// depend on unmodelled bit-reader and allocator callees).

/// Sync state mirroring the int-array layout used by the retail routine:
/// `[0]` data, `[1]` returned-flag (negative = error), `[2]` fill,
/// `[3]` returned offset, `[4]` unsynced, `[5]` header bytes, `[6]` body bytes.
#[derive(Debug, Clone, Default)]
pub struct W32a79OggSync {
    pub data: Vec<u8>,
    pub error: i32,
    pub fill: i32,
    pub returned: i32,
    pub unsynced: i32,
    pub headerbytes: i32,
    pub bodybytes: i32,
}

/// Page location returned by the sync routine (offsets into `data`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct W32a79OggPage {
    pub header: usize,
    pub header_len: i32,
    pub body: usize,
    pub body_len: i32,
}

/// Ogg CRC-32 table (poly 0x04c11db7, MSB first), as stored in the retail image.
pub fn w32a79_crc_table() -> [u32; 256] {
    let mut t = [0u32; 256];
    for (i, e) in t.iter_mut().enumerate() {
        let mut r = (i as u32) << 24;
        for _ in 0..8 {
            r = if r & 0x8000_0000 != 0 {
                (r << 1) ^ 0x04c1_1db7
            } else {
                r << 1
            };
        }
        *e = r;
    }
    t
}

/// `0x009f2620`: ogg_sync_pageseek.  Returns the page length when a valid
/// page is available (filling `page`), a negative skip count when bytes had
/// to be dropped, and 0 when more data is needed.
pub fn w32a79_ogg_sync_pageseek(oy: &mut W32a79OggSync, page: Option<&mut W32a79OggPage>) -> i32 {
    if oy.error < 0 {
        return 0;
    }
    let base = oy.returned as usize;
    let bytes = oy.fill - oy.returned;
    let mut resync = false;
    if oy.headerbytes == 0 {
        if bytes < 0x1b {
            return 0;
        }
        if &oy.data[base..base + 4] != b"OggS" {
            resync = true;
        } else {
            let segs = oy.data[base + 0x1a] as i32;
            let headerbytes = segs + 0x1b;
            if bytes < headerbytes {
                return 0;
            }
            for i in 0..segs as usize {
                oy.bodybytes += oy.data[base + 0x1b + i] as i32;
            }
            oy.headerbytes = headerbytes;
        }
    }
    if !resync {
        if oy.bodybytes + oy.headerbytes > bytes {
            return 0;
        }
        let hb = oy.headerbytes as usize;
        let bb = oy.bodybytes as usize;
        let table = w32a79_crc_table();
        let saved = u32::from_le_bytes(oy.data[base + 0x16..base + 0x1a].try_into().unwrap());
        oy.data[base + 0x16..base + 0x1a].fill(0);
        let mut crc = 0u32;
        for &b in &oy.data[base..base + hb + bb] {
            crc = (crc << 8) ^ table[((b as u32) ^ (crc >> 24)) as usize];
        }
        if crc == saved {
            oy.data[base + 0x16..base + 0x1a].copy_from_slice(&crc.to_le_bytes());
            if let Some(p) = page {
                p.header = base;
                p.header_len = oy.headerbytes;
                p.body = base + hb;
                p.body_len = oy.bodybytes;
            }
            oy.unsynced = 0;
            let total = oy.headerbytes + oy.bodybytes;
            oy.headerbytes = 0;
            oy.bodybytes = 0;
            oy.returned += total;
            return total;
        }
        oy.data[base + 0x16..base + 0x1a].copy_from_slice(&saved.to_le_bytes());
    }
    oy.headerbytes = 0;
    oy.bodybytes = 0;
    let next = oy.data[base + 1..base + 1 + (bytes - 1) as usize]
        .iter()
        .position(|&b| b == b'O')
        .map(|p| base + 1 + p)
        .unwrap_or(oy.fill as usize);
    oy.returned = next as i32;
    base as i32 - next as i32
}

#[cfg(test)]
mod tests_wave32_agent79 {
    use super::*;

    fn make_page(body: &[u8]) -> Vec<u8> {
        let mut v = b"OggS".to_vec();
        v.resize(0x1a, 0);
        v.push(1);
        v.push(body.len() as u8);
        v.extend_from_slice(body);
        let t = w32a79_crc_table();
        let mut crc = 0u32;
        for &b in &v {
            crc = (crc << 8) ^ t[((b as u32) ^ (crc >> 24)) as usize];
        }
        v[0x16..0x1a].copy_from_slice(&crc.to_le_bytes());
        v
    }

    #[test]
    fn crc_table_known() {
        let t = w32a79_crc_table();
        assert_eq!(t[1], 0x04c1_1db7);
        assert_eq!(t[255], 0xb1f7_40b4);
    }

    #[test]
    fn seek_valid_page_and_resync() {
        let page = make_page(b"hello");
        let mut oy = W32a79OggSync {
            data: page.clone(),
            fill: page.len() as i32,
            ..Default::default()
        };
        let mut p = W32a79OggPage::default();
        assert_eq!(
            w32a79_ogg_sync_pageseek(&mut oy, Some(&mut p)),
            page.len() as i32
        );
        assert_eq!((p.body, p.body_len), (28, 5));
        assert_eq!(oy.returned, page.len() as i32);

        let mut data = b"xxxO".to_vec();
        data.extend_from_slice(&page);
        let mut oy = W32a79OggSync {
            fill: data.len() as i32,
            data,
            ..Default::default()
        };
        assert_eq!(w32a79_ogg_sync_pageseek(&mut oy, None), -3);
        assert_eq!(w32a79_ogg_sync_pageseek(&mut oy, None), -1);
    }

    #[test]
    fn seek_needs_more_and_bad_crc() {
        let mut oy = W32a79OggSync {
            data: vec![0; 10],
            fill: 10,
            ..Default::default()
        };
        assert_eq!(w32a79_ogg_sync_pageseek(&mut oy, None), 0);
        let mut page = make_page(b"hi");
        let n = page.len();
        page[n - 1] ^= 1;
        let mut oy = W32a79OggSync {
            data: page,
            fill: n as i32,
            ..Default::default()
        };
        assert!(w32a79_ogg_sync_pageseek(&mut oy, None) < 0);
        assert_eq!(oy.headerbytes, 0);
    }
}
