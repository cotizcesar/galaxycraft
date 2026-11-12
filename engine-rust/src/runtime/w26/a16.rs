#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave26/agent16.rs
// ===========================================================================
// Wave 26 agent 16: SHA-1 block/finalize, RC4 key schedule, path helpers.

/// `0x0093dc60`: SHA-1 compression of one 64-byte block (big-endian words)
/// into `state` (5 words).  Message schedule rotates by 1; the retail code
/// also wipes its stack copy, which is not observable here.
pub fn w26a16_sha1_block(state: &mut [u32; 5], block: &[u8; 64]) {
    let mut w = [0u32; 80];
    for i in 0..16 {
        w[i] = u32::from_be_bytes([
            block[i * 4],
            block[i * 4 + 1],
            block[i * 4 + 2],
            block[i * 4 + 3],
        ]);
    }
    for i in 16..80 {
        w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
    }
    let [mut a, mut b, mut c, mut d, mut e] = *state;
    for (i, wi) in w.iter().enumerate() {
        let (f, k) = match i / 20 {
            0 => ((!b & d) | (c & b), 0x5a82_7999u32),
            1 => (d ^ c ^ b, 0x6ed9_eba1),
            2 => (((d | b) & c) | (d & b), 0x8f1b_bcdc),
            _ => (d ^ c ^ b, 0xca62_c1d6),
        };
        let t = e
            .wrapping_add(a.rotate_left(5))
            .wrapping_add(f)
            .wrapping_add(k)
            .wrapping_add(*wi);
        e = d;
        d = c;
        c = b.rotate_left(30);
        b = a;
        a = t;
    }
    state[0] = state[0].wrapping_add(a);
    state[1] = state[1].wrapping_add(b);
    state[2] = state[2].wrapping_add(c);
    state[3] = state[3].wrapping_add(d);
    state[4] = state[4].wrapping_add(e);
}

/// `0x0093db50`: SHA-1 finalize.  `ctx` is `[count_lo, count_hi, h0..h4]`
/// (count_lo is the word used for the pad length).  `pad` is the retail
/// padding table (0x80 then zeros); `update` is the block-update routine.
/// Pads `((-(lo >> 3) - 9) & 0x3f) + 1` bytes, appends the 8 count bytes
/// (hi word big-endian first, then lo) and writes the digest big-endian.
pub fn w26a16_sha1_final(ctx: &[u32; 7], pad: &[u8], mut update: impl FnMut(&[u8])) -> [u8; 20] {
    let lo = ctx[0];
    let hi = ctx[1];
    let mut count = [0u8; 8];
    count[..4].copy_from_slice(&hi.to_be_bytes());
    count[4..].copy_from_slice(&lo.to_be_bytes());
    let n = ((0u32.wrapping_sub(lo >> 3).wrapping_sub(9)) & 0x3f) as usize + 1;
    update(&pad[..n.min(pad.len())]);
    update(&count);
    let mut out = [0u8; 20];
    for i in 0..5 {
        out[i * 4..i * 4 + 4].copy_from_slice(&ctx[2 + i].to_be_bytes());
    }
    out
}

/// `0x0093e2b0`: RC4 key schedule.  Returns `None` (retail raises error
/// 0x57) for an empty key; otherwise `s[0..256]` permuted, `s[256]` and
/// `s[257]` zero.
pub fn w26a16_rc4_init(key: &[u8]) -> Option<[u8; 258]> {
    if key.is_empty() {
        return None;
    }
    let mut s = [0u8; 258];
    for (i, v) in s.iter_mut().take(256).enumerate() {
        *v = i as u8;
    }
    let mut j: u8 = 0;
    for i in 0..256 {
        let t = s[i];
        j = j.wrapping_add(key[i % key.len()]).wrapping_add(t);
        s[i] = s[j as usize];
        s[j as usize] = t;
    }
    Some(s)
}

/// `0x0093f200`: offset of the file-name part (after the last `/` or `\`).
pub fn w26a16_basename_offset(path: &[u8]) -> usize {
    let mut off = 0;
    for (i, &c) in path.iter().enumerate() {
        if c == 0 {
            break;
        }
        if c == b'/' || c == b'\\' {
            off = i + 1;
        }
    }
    off
}

/// `0x0093f260`: copies the file-name part of `path` into `dst`, at most
/// `dst.len() - 1` bytes (copy only; the terminator is not written by the
/// retail routine).  Returns the number of bytes copied.
pub fn w26a16_copy_basename(path: &[u8], dst: &mut [u8]) -> usize {
    if dst.is_empty() {
        return 0;
    }
    let off = w26a16_basename_offset(path);
    let rest = &path[off..];
    let len = rest.iter().position(|&c| c == 0).unwrap_or(rest.len());
    let n = len.min(dst.len() - 1);
    dst[..n].copy_from_slice(&rest[..n]);
    n
}

/// `0x0093f360`: true when the callee result is zero.
pub fn w26a16_is_zero_result(callee_result: i32) -> bool {
    callee_result == 0
}

/// `0x0093f420`: validates two handles; if either is null it raises error
/// 0x57 (returned as `Err`), else calls the OS routine.
pub fn w26a16_two_handle_call(
    a: usize,
    b: usize,
    mut call: impl FnMut(usize, usize),
) -> Result<(), u32> {
    if a != 0 && b != 0 {
        call(a, b);
        Ok(())
    } else {
        Err(0x57)
    }
}

#[cfg(test)]
mod tests_wave26_agent16 {
    use super::*;

    fn sha1(msg: &[u8]) -> [u8; 20] {
        let mut st = [
            0x6745_2301u32,
            0xefcd_ab89,
            0x98ba_dcfe,
            0x1032_5476,
            0xc3d2_e1f0,
        ];
        let mut data = msg.to_vec();
        data.push(0x80);
        while data.len() % 64 != 56 {
            data.push(0);
        }
        data.extend_from_slice(&((msg.len() as u64) * 8).to_be_bytes());
        for ch in data.chunks(64) {
            let mut b = [0u8; 64];
            b.copy_from_slice(ch);
            w26a16_sha1_block(&mut st, &b);
        }
        let mut out = [0u8; 20];
        for i in 0..5 {
            out[i * 4..i * 4 + 4].copy_from_slice(&st[i].to_be_bytes());
        }
        out
    }

    #[test]
    fn sha1_abc() {
        let d = sha1(b"abc");
        let hex: String = d.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, "a9993e364706816aba3e25717850c26c9cd0d89d");
    }

    #[test]
    fn sha1_final_layout() {
        let ctx = [24u32, 0, 1, 2, 3, 4, 5];
        let mut pad = [0u8; 64];
        pad[0] = 0x80;
        let mut chunks: Vec<Vec<u8>> = Vec::new();
        let out = w26a16_sha1_final(&ctx, &pad, |c| chunks.push(c.to_vec()));
        assert_eq!(
            chunks[0].len(),
            ((0u32.wrapping_sub(3).wrapping_sub(9)) & 0x3f) as usize + 1
        );
        assert_eq!(chunks[1], vec![0, 0, 0, 0, 0, 0, 0, 24]);
        assert_eq!(&out[..4], &[0, 0, 0, 1]);
        assert_eq!(&out[16..], &[0, 0, 0, 5]);
    }

    #[test]
    fn rc4_ksa() {
        assert!(w26a16_rc4_init(&[]).is_none());
        let s = w26a16_rc4_init(b"Key").unwrap();
        // keystream check: RC4("Key", "Plaintext") = BBF316E8D940AF0AD3
        let mut st = s;
        let (mut x, mut y) = (0usize, 0usize);
        let mut out = Vec::new();
        for &p in b"Plaintext" {
            x = (x + 1) & 255;
            y = (y + st[x] as usize) & 255;
            st.swap(x, y);
            let k = st[(st[x] as usize + st[y] as usize) & 255];
            out.push(p ^ k);
        }
        assert_eq!(
            out,
            vec![0xbb, 0xf3, 0x16, 0xe8, 0xd9, 0x40, 0xaf, 0x0a, 0xd3]
        );
        assert_eq!(s[256], 0);
        let mut sorted: Vec<u8> = s[..256].to_vec();
        sorted.sort();
        assert!(sorted.iter().enumerate().all(|(i, &v)| v as usize == i));
    }

    #[test]
    fn paths() {
        assert_eq!(w26a16_basename_offset(b"a\\b/c.txt\0"), 4);
        assert_eq!(w26a16_basename_offset(b"plain"), 0);
        let mut d = [0u8; 4];
        assert_eq!(w26a16_copy_basename(b"x/hello\0", &mut d), 3);
        assert_eq!(&d[..3], b"hel");
        assert_eq!(w26a16_copy_basename(b"x", &mut []), 0);
    }

    #[test]
    fn small() {
        assert!(w26a16_is_zero_result(0));
        assert!(!w26a16_is_zero_result(1));
        let mut n = 0;
        assert_eq!(w26a16_two_handle_call(0, 1, |_, _| n += 1), Err(0x57));
        assert_eq!(w26a16_two_handle_call(1, 2, |_, _| n += 1), Ok(()));
        assert_eq!(n, 1);
    }
}
