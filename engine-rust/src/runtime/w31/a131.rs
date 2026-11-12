#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave31/agent131.rs
// ===========================================================================
// Wave 31 agent 131: MSVC CRT scanf/stdio argument-pointer helpers.

/// EINVAL as stored into errno by the CRT helpers.
pub const W31A131_EINVAL: u32 = 0x16;

/// `0x00c9f7e7`: stores a 32-bit value through the next va_list pointer
/// argument.  `args` is the va_list of pointer slots, `cursor` the index
/// advanced by one slot; a null slot yields `Err(EINVAL)`.
pub fn w31a131_store_u32(
    args: &[Option<usize>],
    cursor: &mut usize,
    value: u32,
    mem: &mut std::collections::HashMap<usize, u32>,
) -> Result<(), u32> {
    let slot = args.get(*cursor).copied().flatten();
    *cursor += 1;
    match slot {
        None => Err(W31A131_EINVAL),
        Some(p) => {
            mem.insert(p, value);
            Ok(())
        }
    }
}

/// `0x00c9f81b`: stores a 64-bit value (two dwords, low first) through the
/// next va_list pointer argument; null slot yields `Err(EINVAL)`.
pub fn w31a131_store_u64(
    args: &[Option<usize>],
    cursor: &mut usize,
    value: u64,
    mem: &mut std::collections::HashMap<usize, u32>,
) -> Result<(), u32> {
    let slot = args.get(*cursor).copied().flatten();
    *cursor += 1;
    match slot {
        None => Err(W31A131_EINVAL),
        Some(p) => {
            mem.insert(p, value as u32);
            mem.insert(p + 4, (value >> 32) as u32);
            Ok(())
        }
    }
}

/// `0x00ca040a`: validates that the stream's `+8` field (buffer) is
/// non-zero.
pub fn w31a131_check_nonzero_field8(field8: u32) -> Result<(), u32> {
    if field8 == 0 {
        Err(W31A131_EINVAL)
    } else {
        Ok(())
    }
}

/// `0x00ca0426`: validates that fields `+0` and `+0x18` are both non-zero.
pub fn w31a131_check_fields_0_and_18(field0: u32, field18: u32) -> Result<(), u32> {
    if field0 != 0 && field18 != 0 {
        Ok(())
    } else {
        Err(W31A131_EINVAL)
    }
}

/// `0x00ca0447`: validates that `+8` is non-zero and `<= +4`.
pub fn w31a131_check_range_8_le_4(field4: u32, field8: u32) -> Result<(), u32> {
    if field8 != 0 && field8 <= field4 {
        Ok(())
    } else {
        Err(W31A131_EINVAL)
    }
}

/// `0x00ca04d8`: stores an integer of `size` bytes (1, 2, 4 or 8) through
/// the next va_list pointer; null slot or other size returns `false`
/// (null slot also records EINVAL, returned as `Err`).
pub fn w31a131_store_sized(
    args: &[Option<usize>],
    cursor: &mut usize,
    size: u32,
    lo: u32,
    hi: u32,
    mem: &mut std::collections::HashMap<usize, u32>,
) -> Result<bool, u32> {
    let slot = args.get(*cursor).copied().flatten();
    *cursor += 1;
    let Some(p) = slot else {
        return Err(W31A131_EINVAL);
    };
    match size {
        1 => {
            mem.insert(p, lo & 0xff);
        }
        2 => {
            mem.insert(p, lo & 0xffff);
        }
        4 => {
            mem.insert(p, lo);
        }
        8 => {
            mem.insert(p, lo);
            mem.insert(p + 4, hi);
        }
        _ => return Ok(false),
    }
    Ok(true)
}

/// `0x00ca06ce`: unsupported-signal filter; returns `(result, errno)` where
/// result is always `-1` and errno is EINVAL unless `sig` is 1, 3, 0xd,
/// 0x10 or 0x11.
pub fn w31a131_signal_filter(sig: i32) -> (i32, Option<u32>) {
    let ok = sig == 1 || sig == 3 || sig == 0xd || (0x10..=0x11).contains(&sig);
    (-1, if ok { None } else { Some(W31A131_EINVAL) })
}

/// `0x00ca0b90`: `strncmp` over byte slices (bytes past the end read as
/// zero); returns 0, 1 or `u32::MAX` (-1).
pub fn w31a131_strncmp(a: &[u8], b: &[u8], n: usize) -> i32 {
    for i in 0..n {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x != y {
            return if x < y { -1 } else { 1 };
        }
        if x == 0 {
            return 0;
        }
    }
    0
}

/// `0x00ca0f24`: open-wrapper; `open` is the callee taking flag `0x80`.
/// Returns `Ok(handle)` stored through the out pointer, or the errno.
pub fn w31a131_open_wrapper(
    out: Option<&mut u32>,
    open: impl FnOnce(u32) -> Result<u32, u32>,
) -> u32 {
    match out {
        None => W31A131_EINVAL,
        Some(o) => match open(0x80) {
            Ok(h) => {
                *o = h;
                0
            }
            Err(e) => {
                *o = 0;
                e
            }
        },
    }
}

/// `0x00ca0f6b`: forwards to the shared open helper with mode `0x40`.
pub fn w31a131_forward_mode40<R>(f: impl FnOnce(u32) -> R) -> R {
    f(0x40)
}

#[cfg(test)]
mod tests_wave31_agent131 {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn stores() {
        let args = [Some(0x100usize), None, Some(0x200)];
        let mut m = HashMap::new();
        let mut c = 0;
        assert_eq!(w31a131_store_u32(&args, &mut c, 7, &mut m), Ok(()));
        assert_eq!(w31a131_store_u32(&args, &mut c, 7, &mut m), Err(0x16));
        assert_eq!(
            w31a131_store_u64(&args, &mut c, 0x1_0000_0002, &mut m),
            Ok(())
        );
        assert_eq!(m[&0x200], 2);
        assert_eq!(m[&0x204], 1);
        let mut c = 0;
        assert_eq!(
            w31a131_store_sized(&args, &mut c, 2, 0x12345, 0, &mut m),
            Ok(true)
        );
        assert_eq!(m[&0x100], 0x2345);
        let mut c = 2;
        assert_eq!(
            w31a131_store_sized(&args, &mut c, 3, 0, 0, &mut m),
            Ok(false)
        );
    }

    #[test]
    fn checks() {
        assert!(w31a131_check_nonzero_field8(1).is_ok());
        assert!(w31a131_check_nonzero_field8(0).is_err());
        assert!(w31a131_check_fields_0_and_18(1, 1).is_ok());
        assert!(w31a131_check_fields_0_and_18(1, 0).is_err());
        assert!(w31a131_check_range_8_le_4(4, 4).is_ok());
        assert!(w31a131_check_range_8_le_4(4, 5).is_err());
        assert!(w31a131_check_range_8_le_4(4, 0).is_err());
        assert_eq!(w31a131_signal_filter(3), (-1, None));
        assert_eq!(w31a131_signal_filter(2), (-1, Some(0x16)));
    }

    #[test]
    fn strings_and_open() {
        assert_eq!(w31a131_strncmp(b"abc\0", b"abd\0", 3), -1);
        assert_eq!(w31a131_strncmp(b"abd", b"abc", 3), 1);
        assert_eq!(w31a131_strncmp(b"abc", b"abx", 2), 0);
        assert_eq!(w31a131_strncmp(b"a\0x", b"a\0y", 3), 0);
        let mut h = 9;
        assert_eq!(
            w31a131_open_wrapper(Some(&mut h), |m| {
                assert_eq!(m, 0x80);
                Ok(5)
            }),
            0
        );
        assert_eq!(h, 5);
        assert_eq!(w31a131_open_wrapper(Some(&mut h), |_| Err(2)), 2);
        assert_eq!(w31a131_open_wrapper(None, |_| Ok(1)), 0x16);
        assert_eq!(w31a131_forward_mode40(|m| m), 0x40);
    }
}
